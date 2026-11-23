//! Installed bundles carry their service runtime; developer builds can use PATH.
use std::{
    ffi::OsString,
    path::{Path, PathBuf},
};

pub fn binary() -> OsString {
    if let Some(explicit) = std::env::var_os("TODO_NODE_BINARY") {
        return explicit;
    }
    std::env::current_exe()
        .ok()
        .into_iter()
        .flat_map(|executable| candidates(&executable))
        .find(|candidate| candidate.is_file())
        .map(PathBuf::into_os_string)
        .unwrap_or_else(|| "node".into())
}

fn candidates(executable: &Path) -> Vec<PathBuf> {
    let Some(parent) = executable.parent() else {
        return Vec::new();
    };
    let name = if cfg!(windows) { "node.exe" } else { "node" };
    let mut candidates = vec![parent.join("runtime").join(name)];
    if parent.file_name().is_some_and(|name| name == "MacOS") {
        if let Some(contents) = parent.parent() {
            candidates.push(contents.join("Resources/runtime").join(name));
        }
    }
    candidates
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn portable_and_application_bundle_layouts_do_not_search_the_working_directory() {
        let name = if cfg!(windows) { "node.exe" } else { "node" };
        let portable = PathBuf::from("installed/To Do/todo-desktop");
        assert_eq!(
            candidates(&portable),
            [PathBuf::from("installed/To Do/runtime").join(name)]
        );
        let app = PathBuf::from("installed/To Do.app/Contents/MacOS/todo-desktop");
        assert_eq!(
            candidates(&app)[1],
            PathBuf::from("installed/To Do.app/Contents/Resources/runtime").join(name)
        );
    }
}
