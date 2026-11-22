fn main() {
    // Scope the cap to this disposable parser module; the UI/Skia build has its
    // own memory requirements. A corrupt allocation traps inside the worker.
    if std::env::var("TARGET").as_deref() == Ok("wasm32-unknown-unknown") {
        println!("cargo:rustc-link-arg-cdylib=--max-memory=536870912");
    }
}
