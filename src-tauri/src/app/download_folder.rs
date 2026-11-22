//! Native download-folder picker.
//!
//! The build-time `--download-dir` is fixed into the app, so a user who wants
//! downloads somewhere else had no way to say so. This shows macOS's own folder
//! chooser and remembers the answer next to the app's other runtime settings, so
//! the choice survives restarts without rebuilding the app.

/// Ask the user for a download folder. `None` means cancelled.
///
/// NSOpenPanel must be created and run on the main thread; callers reach this
/// through `run_on_main_thread`.
#[cfg(target_os = "macos")]
pub fn pick_folder(start: Option<&std::path::Path>) -> Option<std::path::PathBuf> {
    use objc2_app_kit::{NSModalResponseOK, NSOpenPanel};
    use objc2_foundation::{MainThreadMarker, NSString, NSURL};

    let mtm = MainThreadMarker::new()?;
    let panel = NSOpenPanel::openPanel(mtm);
    panel.setCanChooseFiles(false);
    panel.setCanChooseDirectories(true);
    panel.setAllowsMultipleSelection(false);
    panel.setCanCreateDirectories(true);
    panel.setPrompt(Some(&NSString::from_str("Choose")));
    panel.setMessage(Some(&NSString::from_str(
        "Where should downloads be saved?",
    )));
    if let Some(start) = start {
        let path = start.to_string_lossy();
        let url = NSURL::fileURLWithPath(&NSString::from_str(&path));
        panel.setDirectoryURL(Some(&url));
    }

    if panel.runModal() != NSModalResponseOK {
        return None;
    }
    let url = panel.URLs().firstObject()?;
    url.path()
        .map(|path| std::path::PathBuf::from(path.to_string()))
}

#[cfg(not(target_os = "macos"))]
pub fn pick_folder(_start: Option<&std::path::Path>) -> Option<std::path::PathBuf> {
    None
}

#[cfg(test)]
mod tests {
    use crate::util::{read_download_dir, write_download_dir};
    use std::path::PathBuf;

    fn temp_file(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("pake-download-dir-{}-{name}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir.join("download-dir.txt")
    }

    #[test]
    fn a_picked_folder_round_trips() {
        let path = temp_file("round-trip");
        let picked = PathBuf::from("/Users/example/Downloads");
        write_download_dir(&path, &picked).unwrap();
        assert_eq!(read_download_dir(&path), Some(picked));
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn relative_or_empty_settings_are_ignored() {
        // A hand-edited settings file must not redirect downloads to a path
        // that resolves against whatever the process happens to consider cwd.
        for value in ["Downloads", "", "   ", "./relative"] {
            let path = temp_file("rejects");
            std::fs::write(&path, value).unwrap();
            assert_eq!(read_download_dir(&path), None, "{value:?}");
            let _ = std::fs::remove_dir_all(path.parent().unwrap());
        }
    }

    #[test]
    fn a_missing_settings_file_means_no_override() {
        let path = temp_file("absent").with_file_name("does-not-exist.txt");
        assert_eq!(read_download_dir(&path), None);
    }
}
