use std::path::Path;
use std::process::Command;

// Writing the `com.apple.metadata:kMDItemFinderComment` extended attribute directly
// does NOT reliably surface in Finder's Comments column / Get Info on current macOS:
// Finder appears to keep its own authoritative store and only mirrors it back to that
// xattr when Finder itself performs the write. Going through Finder via Apple Events
// (AppleScript) is the only method verified to actually work end-to-end for *writes*.
// The first write triggers a one-time macOS prompt asking to allow this app to
// control "Finder" (Automation permission) - it must be accepted.
//
// Reads are the opposite trade-off: once Finder has written a comment, it does
// mirror it into that same extended attribute, so we can read it directly
// without spawning a process - much cheaper, which matters since a full
// rescan reads every matching file's current comment to decide whether a
// write is even needed.
const ATTR_NAME: &str = "com.apple.metadata:kMDItemFinderComment";

fn escape_applescript_string(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

fn run_osascript(script: &str) -> std::io::Result<String> {
    let output = Command::new("osascript").arg("-e").arg(script).output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        return Err(std::io::Error::other(stderr));
    }
    Ok(String::from_utf8_lossy(&output.stdout)
        .trim_end_matches('\n')
        .to_string())
}

/// Reads the current Finder comment of a file, if any, straight from the
/// extended attribute (no subprocess).
pub fn get_comment(path: &Path) -> Option<String> {
    let bytes = xattr::get(path, ATTR_NAME).ok().flatten()?;
    let value: plist::Value = plist::from_bytes(&bytes).ok()?;
    value.into_string()
}

/// Writes (overwriting) the Finder comment of a file.
pub fn set_comment(path: &Path, comment: &str) -> std::io::Result<()> {
    let posix_path = escape_applescript_string(&path.to_string_lossy());
    let escaped_comment = escape_applescript_string(comment);
    let script = format!(
        "tell application \"Finder\" to set comment of (POSIX file \"{}\" as alias) to \"{}\"",
        posix_path, escaped_comment
    );
    run_osascript(&script).map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "requires an interactive macOS session with Finder Automation permission granted; not available on headless CI runners"]
    fn roundtrip_via_finder_apple_events() {
        let dir = std::env::temp_dir().join(format!("spine_test_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file_path = dir.join("Volume 6 - Épisode's \"test\".cbz");
        std::fs::write(&file_path, b"fake cbz content").unwrap();

        assert_eq!(get_comment(&file_path), None);

        set_comment(&file_path, "Volume 6").unwrap();
        assert_eq!(get_comment(&file_path), Some("Volume 6".to_string()));

        set_comment(
            &file_path,
            "Volume 6 | old comment with \"quotes\" and \\backslash",
        )
        .unwrap();
        assert_eq!(
            get_comment(&file_path),
            Some("Volume 6 | old comment with \"quotes\" and \\backslash".to_string())
        );

        std::fs::remove_dir_all(&dir).ok();
    }
}
