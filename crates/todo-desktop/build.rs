use std::{env, fs, path::Path};

fn collect(root: &Path, directory: &Path, prefix: &str, entries: &mut Vec<(String, String)>) {
    println!("cargo:rerun-if-changed={}", directory.display());
    let mut paths = fs::read_dir(directory)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect::<Vec<_>>();
    paths.sort();
    for path in paths {
        if path.is_dir() {
            collect(root, &path, prefix, entries);
        } else if matches!(
            path.extension().and_then(|value| value.to_str()),
            Some("js" | "mjs" | "json" | "txt")
        ) {
            let name = format!(
                "{prefix}/{}",
                path.strip_prefix(root)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/")
            );
            entries.push((
                name,
                path.canonicalize().unwrap().to_string_lossy().into_owned(),
            ));
        }
    }
}

fn main() {
    let mut entries = Vec::new();
    let web = Path::new("../todo-browser/wwwroot");
    for name in ["notes.js", "backgrounds.js"] {
        let path = web.join(name);
        println!("cargo:rerun-if-changed={}", path.display());
        entries.push((
            format!("web/{name}"),
            path.canonicalize().unwrap().to_string_lossy().into_owned(),
        ));
    }
    collect(web, &web.join("cloud"), "web", &mut entries);
    collect(web, &web.join("backup"), "web", &mut entries);
    if web.join("vendor/pst-writer").is_dir() {
        collect(web, &web.join("vendor/pst-writer"), "web", &mut entries);
    }
    collect(
        Path::new("native"),
        Path::new("native"),
        "native",
        &mut entries,
    );
    let mut source = String::from("pub const ASSETS: &[(&str, &[u8])] = &[\n");
    for (name, path) in entries {
        source.push_str(&format!("({name:?}, include_bytes!({path:?})),\n"));
    }
    source.push_str("];\n");
    fs::write(
        Path::new(&env::var("OUT_DIR").unwrap()).join("native_assets.rs"),
        source,
    )
    .unwrap();
}
