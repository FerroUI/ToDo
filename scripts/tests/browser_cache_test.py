"""Exercise cache identities and filesystem freshness in isolated, real Git fixtures."""
import importlib.util
import os
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch

SCRIPT = Path(__file__).resolve().parents[1] / "ci-browser-cache.py"
spec = importlib.util.spec_from_file_location("browser_cache", SCRIPT)
cache = importlib.util.module_from_spec(spec)
spec.loader.exec_module(cache)


class BrowserCacheTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.framework = self.root / "FerroUI"
        self.framework.mkdir()
        self.write("FerroUI/Cargo.toml", 'wasm-bindgen = "=0.2.129"\n')
        self.write("FerroUI/scripts/browser/setup.sh", 'EMSDK_VERSION="3.1.74"\nRUST_VERSION="nightly"\n')
        for directory in [self.framework, self.root]:
            self.git(directory, "init", "--quiet")
            self.git(directory, "config", "user.name", "Cache test")
            self.git(directory, "config", "user.email", "cache@example.invalid")
        self.git(self.framework, "add", ".")
        self.git(self.framework, "commit", "--quiet", "-m", "Framework")
        self.pin = self.git(self.framework, "rev-parse", "HEAD").strip()
        self.files = {
            "Cargo.toml": "[workspace]\n",
            "Cargo.lock": "version = 4\n",
            ".cargo/config.toml": "[build]\n",
            "scripts/build-browser.sh": "cargo build\n",
            "scripts/ci-browser-cache.py": SCRIPT.read_text(),
            ".github/actions/browser-toolchain/action.yml": "name: tools\n",
            "crates/example/Cargo.toml": '[package]\nname = "example"\n',
            "crates/example/build.rs": "fn main() {}\n",
            "crates/example/src/lib.rs": "pub fn value() -> u8 { 1 }\n",
            "crates/example/assets/theme.xaml": "<ResourceDictionary />\n",
            "crates/example/wwwroot/main.js": "export const runtime = 1;\n",
            "scripts/ui/scenario.mjs": "export const gate = 1;\n",
        }
        for name, content in self.files.items():
            self.write(name, content)
        self.git(self.root, "add", "Cargo.toml", "Cargo.lock", ".cargo", "scripts", ".github", "crates")
        self.git(self.root, "update-index", "--add", "--cacheinfo", f"160000,{self.pin},FerroUI")
        self.git(self.root, "commit", "--quiet", "-m", "Application")
        self.addCleanup(patch.stopall)
        patch.object(cache, "ROOT", self.root).start()
        patch.object(cache, "FRAMEWORK", self.framework).start()
        patch.dict(os.environ, {"GITHUB_ACTIONS": "true", "RUNNER_OS": "Linux", "RUNNER_ARCH": "X64"}).start()
        check_output = subprocess.check_output
        patch.object(cache.subprocess, "check_output", side_effect=lambda command, **kwargs:
            b"rustc test compiler\n" if command == ["rustc", "-vV"] else check_output(command, **kwargs)).start()

    @staticmethod
    def git(directory, *args):
        return subprocess.check_output(["git", "-C", str(directory), *args], text=True)

    def write(self, name, content):
        path = self.root / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(content)
        return path

    def key(self):
        return cache.cache_key(self.pin)

    def test_rust_theme_manifest_and_build_inputs_change_exact_key(self):
        original = self.key()
        for name in ["crates/example/src/lib.rs", "crates/example/assets/theme.xaml",
                     "crates/example/Cargo.toml", "crates/example/build.rs", ".cargo/config.toml"]:
            with self.subTest(name=name):
                self.write(name, self.files[name] + "changed\n")
                changed = self.key()
                self.assertNotEqual(changed, original)
                self.assertEqual(changed.rsplit("-", 1)[0], original.rsplit("-", 1)[0])
                self.write(name, self.files[name])
        self.write("crates/example/src/new.rs", "pub struct Added;\n")
        self.git(self.root, "add", "crates/example/src/new.rs")
        self.assertNotEqual(self.key(), original)

    def test_qa_runtime_and_untracked_output_do_not_change_key(self):
        original = self.key()
        self.write("scripts/ui/scenario.mjs", "different QA\n")
        self.write("crates/example/wwwroot/main.js", "different runtime\n")
        self.write("crates/example/target/generated.rs", "untracked generated output\n")
        self.assertEqual(self.key(), original)

    def test_exact_hit_normalizes_only_clean_tracked_compiler_inputs(self):
        key = self.key()
        untouched = [self.write("crates/example/target/generated.rs", "generated\n"),
                     self.write("crates/example/src/untracked.txt", "untracked\n"),
                     self.root / "crates/example/wwwroot/main.js", self.root / ".git/HEAD"]
        before = {path: path.stat().st_mtime_ns for path in untouched}
        source = self.root / "crates/example/src/lib.rs"
        os.utime(source, ns=(source.stat().st_atime_ns, 2_000_000_000_000_000_000))
        self.assertTrue(cache.restore_app("true", key, key))
        timestamp = int(self.git(self.framework, "show", "-s", "--format=%ct", "HEAD")) * 1_000_000_000
        self.assertEqual(source.stat().st_mtime_ns, timestamp)
        self.assertEqual(source.parent.stat().st_mtime_ns, timestamp)
        for path in untouched:
            self.assertEqual(path.stat().st_mtime_ns, before[path], str(path))

    def test_fallback_mismatch_and_dirty_sources_never_normalize(self):
        key = self.key()
        source = self.root / "crates/example/src/lib.rs"
        before = source.stat().st_mtime_ns
        with self.assertRaisesRegex(RuntimeError, "does not match"):
            cache.restore_app("true", key, key + "-different")
        self.assertEqual(source.stat().st_mtime_ns, before)
        self.write("crates/example/src/lib.rs", "changed application source\n")
        before = source.stat().st_mtime_ns
        self.assertFalse(cache.restore_app("false", self.key(), key))
        self.assertEqual(source.stat().st_mtime_ns, before)
        with self.assertRaisesRegex(RuntimeError, "does not match"):
            cache.restore_app("true", key, key)
        self.assertEqual(source.stat().st_mtime_ns, before)
        changed = self.key()
        with self.assertRaises(subprocess.CalledProcessError):
            cache.restore_app("true", changed, changed)
        self.assertEqual(source.stat().st_mtime_ns, before)


if __name__ == "__main__":
    unittest.main()
