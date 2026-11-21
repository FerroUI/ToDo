use std::io::Write;
use std::path::{Path, PathBuf};
use todo_domain::AppState;

pub fn data_directory() -> PathBuf {
    if let Some(path) = std::env::var_os("TODO_DATA_DIR") {
        return path.into();
    }
    #[cfg(target_os = "windows")]
    let base = std::env::var_os("LOCALAPPDATA").map(PathBuf::from);
    #[cfg(target_os = "macos")]
    let base = std::env::var_os("HOME")
        .map(|home| PathBuf::from(home).join("Library/Application Support"));
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    let base = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/share")));
    base.unwrap_or_else(std::env::temp_dir).join("FerroUI/ToDo")
}

pub fn load(directory: &Path) -> (AppState, Option<String>) {
    let file = directory.join("tasks.json");
    match std::fs::read_to_string(&file) {
        Ok(json) => match AppState::from_json(&json) {
            Ok(state) => (state, None),
            Err(error) => {
                let recovery = directory.join(format!(
                    "tasks-recovery-{}.json",
                    chrono::Local::now().format("%Y%m%d-%H%M%S")
                ));
                let copied = std::fs::copy(&file, &recovery).is_ok();
                (
                    AppState::new(),
                    Some(format!(
                        "Saved tasks could not be read. {} {error}",
                        if copied {
                            "A recovery copy was saved."
                        } else {
                            "Copy tasks.json before saving new tasks."
                        }
                    )),
                )
            }
        },
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => (AppState::new(), None),
        Err(error) => (
            AppState::new(),
            Some(format!("Could not open saved tasks: {error}")),
        ),
    }
}

pub fn save(directory: &Path, json: &str) -> Result<(), String> {
    let save = || -> std::io::Result<()> {
        std::fs::create_dir_all(directory)?;
        let temporary = directory.join("tasks.json.tmp");
        let mut file = std::fs::File::create(&temporary)?;
        file.write_all(json.as_bytes())?;
        file.sync_all()?;
        drop(file);
        std::fs::rename(temporary, directory.join("tasks.json"))
    };
    save().map_err(|error| format!("Could not save tasks: {error}"))
}
