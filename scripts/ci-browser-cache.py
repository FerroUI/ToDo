#!/usr/bin/env python3
"""Key compiled browser artifacts and make clean pinned path sources reproducible."""

import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess

ROOT = Path(__file__).resolve().parent.parent
FRAMEWORK = ROOT / "FerroUI"
CACHE_PATHS = "\n".join(
    (
        "target/browser/.fingerprint",
        "target/browser/build",
        "target/browser/deps",
        "target/wasm32-unknown-emscripten/browser",
    )
)
EXCLUDED = {".git", "node_modules", "target"}


def git(directory, *args):
    return subprocess.check_output(["git", "--no-optional-locks", "-C", str(directory), *args])


def pins():
    setup = (FRAMEWORK / "scripts/browser/setup.sh").read_text()
    manifest = (FRAMEWORK / "Cargo.toml").read_text()
    return {
        "emsdk": re.search(r'^EMSDK_VERSION="([^"]+)"$', setup, re.M)[1],
        "rust": re.search(r'^RUST_VERSION="([^"]+)"$', setup, re.M)[1],
        "wasm-bindgen": re.search(r'^wasm-bindgen = "=([^"]+)"$', manifest, re.M)[1],
    }


def emit(name, value):
    output = os.environ.get("GITHUB_OUTPUT")
    if output:
        with open(output, "a", encoding="utf-8") as stream:
            stream.write(f"{name}<<TODO_CACHE_OUTPUT\n{value}\nTODO_CACHE_OUTPUT\n")
    else:
        print(f"{name}={value}")


def pinned_sources():
    commit = git(FRAMEWORK, "rev-parse", "HEAD").decode().strip()
    expected = git(ROOT, "rev-parse", "HEAD:FerroUI").decode().strip()
    if commit != expected:
        raise RuntimeError("FerroUI must match the application's committed submodule pin")
    subprocess.run(["git", "--no-optional-locks", "-C", str(FRAMEWORK), "diff", "--quiet", "HEAD", "--"], check=True)
    timestamp = int(git(FRAMEWORK, "show", "-s", "--format=%ct", "HEAD")) * 1_000_000_000
    files, directories = [], set()
    for entry in git(FRAMEWORK, "ls-files", "--stage", "-z").split(b"\0"):
        if not entry:
            continue
        metadata, relative = entry.split(b"\t", 1)
        # Nested gitlinks are not framework source files. No recursive walk can
        # accidentally enter their metadata, node_modules or generated output.
        if metadata.split()[0] == b"160000":
            continue
        relative = Path(os.fsdecode(relative))
        if EXCLUDED.intersection(relative.parts):
            continue
        source = FRAMEWORK / relative
        files.append(source)
        directory = source.parent
        while directory != FRAMEWORK:
            directories.add(directory)
            directory = directory.parent
    return commit, timestamp, files, directories


def application_inputs():
    """Include tracked compiler inputs, not the separately staged browser runtime/QA."""
    files = set()
    for entry in git(
        ROOT, "ls-files", "--stage", "-z", "--", "crates", ".cargo",
        "Cargo.toml", "Cargo.lock", "rust-toolchain", "rust-toolchain.toml",
    ).split(b"\0"):
        if not entry:
            continue
        metadata, relative = entry.split(b"\t", 1)
        relative = Path(os.fsdecode(relative))
        if metadata.split()[0] == b"160000" or EXCLUDED.intersection(relative.parts):
            continue
        if "wwwroot" not in relative.parts:
            files.add(ROOT / relative)
    return files


def normalize(files, directories, timestamp):
    for path in [*files, *sorted(directories, key=lambda p: len(p.parts), reverse=True)]:
        stat = path.stat(follow_symlinks=False)
        os.utime(path, ns=(stat.st_atime_ns, timestamp), follow_symlinks=False)


def cache_key(commit):
    digest = hashlib.sha256()
    inputs = {
        ROOT / "Cargo.toml",
        ROOT / "Cargo.lock",
        ROOT / ".cargo/config.toml",
        ROOT / "scripts/build-browser.sh",
        ROOT / "scripts/ci-browser-cache.py",
        ROOT / ".github/actions/browser-toolchain/action.yml",
        *application_inputs(),
    }
    for path in sorted(inputs):
        digest.update(str(path.relative_to(ROOT)).encode() + b"\0")
        digest.update(path.read_bytes())
    digest.update(subprocess.check_output(["rustc", "-vV"], cwd=ROOT))
    environment = {
        name: value
        for name, value in os.environ.items()
        if name.startswith(("CARGO_PROFILE_", "CARGO_TARGET_"))
        or name in {
            "CARGO_ENCODED_RUSTFLAGS", "CARGO_INCREMENTAL", "RUSTFLAGS",
            "RUSTC_WRAPPER", "RUSTC_WORKSPACE_WRAPPER", "EMCC_CFLAGS", "EMSDK",
            "CC", "CXX", "CFLAGS", "CXXFLAGS", "LDFLAGS", "ImageOS",
        }
    }
    digest.update(json.dumps({"pins": pins(), "env": environment}, sort_keys=True).encode())
    return f"todo-browser-v1-{os.environ.get('RUNNER_OS', 'local')}-{os.environ.get('RUNNER_ARCH', 'local')}-{commit}-{digest.hexdigest()[:24]}"


def prepare(dry_run):
    commit, timestamp, files, directories = pinned_sources()
    if not dry_run:
        if os.environ.get("GITHUB_ACTIONS") != "true":
            raise RuntimeError("Source timestamp normalization is CI-only; use --dry-run locally")
        # Cargo compares path-source mtimes to cached dep-info. WinUI's build
        # script also watches source directories, and Cargo mtime_recursive()
        # includes their mtimes. Normalize only clean tracked source files and
        # their derived ancestor directories. Never walk or touch .git,
        # untracked/generated files, node_modules or target contents. The full
        # submodule SHA in the exact cache key prevents reuse across revisions.
        normalize(files, directories, timestamp)
    key = cache_key(commit)
    emit("key", key)
    # Keep canonical toolchain/framework artifacts usable across app manifest and
    # theme-root changes. Cargo still validates compiler, flags, features, build
    # scripts and dependency fingerprints before reusing individual artifacts.
    # The exact key above remains the save key and includes all application inputs.
    emit("restore-prefix", key.rsplit("-", 1)[0] + "-")
    emit("paths", CACHE_PATHS)
    print(f"Pinned source timestamps: {len(files)} files, {len(directories)} source directories; dry-run={dry_run}")


def restore_app(cache_hit, primary_key, matched_key, dry_run=False):
    # A prefix hit may contain a different application. Keep fresh checkout times
    # so Cargo cannot mistake changed content for an older compiled dependency.
    if cache_hit != "true":
        print("Application timestamps unchanged: no exact compilation cache hit")
        return False
    commit, timestamp, _, _ = pinned_sources()
    expected = cache_key(commit)
    if primary_key != expected or matched_key != expected:
        raise RuntimeError("Restored compilation key does not match current compiler inputs")
    files = sorted(application_inputs())
    subprocess.run(
        ["git", "--no-optional-locks", "-C", str(ROOT), "diff", "--quiet", "HEAD", "--",
         *[str(path.relative_to(ROOT)) for path in files]], check=True,
    )
    if not dry_run:
        if os.environ.get("GITHUB_ACTIONS") != "true":
            raise RuntimeError("Source timestamp normalization is CI-only; use --dry-run locally")
        directories = set()
        for path in files:
            directory = path.parent
            while directory != ROOT:
                directories.add(directory)
                directory = directory.parent
        # Exact content equality, not a commit-time heuristic, makes these old
        # timestamps safe. Include watched source directories; leave generated,
        # untracked and runtime files themselves untouched.
        normalize(files, directories, timestamp)
    print(f"Exact application cache verified: {len(files)} compiler inputs; dry-run={dry_run}")
    return True


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", choices=("pins", "prepare", "restore-app"))
    parser.add_argument("--dry-run", action="store_true")
    parser.add_argument("--cache-hit", default="")
    parser.add_argument("--primary-key", default="")
    parser.add_argument("--matched-key", default="")
    arguments = parser.parse_args()
    if arguments.command == "pins":
        for name, value in pins().items():
            emit(name, value)
    elif arguments.command == "prepare":
        prepare(arguments.dry_run)
    else:
        restore_app(arguments.cache_hit, arguments.primary_key, arguments.matched_key, arguments.dry_run)
