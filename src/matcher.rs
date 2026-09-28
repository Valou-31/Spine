use crate::config::{Config, PatternEntry};
use crate::finder_comment;
use regex::Regex;
use std::path::Path;

/// Compiling a regex isn't free, so we compile each enabled pattern once and
/// reuse it for every file instead of recompiling per file/event. `refresh`
/// is cheap to call unconditionally before each use: it only rebuilds when
/// the source patterns actually changed since last time.
#[derive(Default)]
pub struct CompiledPatterns {
    source: Vec<PatternEntry>,
    compiled: Vec<Regex>,
}

impl CompiledPatterns {
    pub fn refresh(&mut self, patterns: &[PatternEntry]) {
        if self.source == patterns {
            return;
        }
        self.compiled = patterns
            .iter()
            .filter(|p| p.enabled)
            .filter_map(|p| Regex::new(&p.regex).ok())
            .collect();
        self.source = patterns.to_vec();
    }

    fn detect_tags(&self, filename: &str) -> Vec<String> {
        self.compiled
            .iter()
            .filter_map(|re| re.find(filename))
            .map(|m| m.as_str().to_string())
            .collect()
    }
}

fn extension_allowed(path: &Path, extensions: &[String]) -> bool {
    match path.extension().and_then(|e| e.to_str()) {
        Some(ext) => extensions
            .iter()
            .any(|allowed| allowed.eq_ignore_ascii_case(ext)),
        None => false,
    }
}

/// Inspects a single file and, if a pattern matches, writes/updates its Finder comment.
/// Returns Some(comment written) on success, None if nothing was done.
pub fn process_file(path: &Path, config: &Config, patterns: &CompiledPatterns) -> Option<String> {
    // Order matters here: extension/regex checks are pure string work with no
    // syscall, while is_file() stats the path. Rejecting on the cheap checks
    // first avoids a stat() for every directory and every irrelevant file
    // when walking a large tree (most entries in a typical library are
    // directories or non-media files like .nfo/.srt).
    let filename = path.file_name()?.to_str()?;
    if !extension_allowed(path, &config.extensions) {
        return None;
    }
    let tags = patterns.detect_tags(filename);
    if tags.is_empty() {
        return None;
    }
    if !path.is_file() {
        return None;
    }
    let new_tag = tags.join(" ");
    let existing = finder_comment::get_comment(path);

    let final_comment = if config.overwrite {
        new_tag
    } else {
        match &existing {
            Some(e) if !e.trim().is_empty() => {
                if e.contains(&new_tag) {
                    return None;
                }
                format!("{} | {}", new_tag, e)
            }
            _ => new_tag,
        }
    };

    // Writing an unchanged comment would still emit a filesystem metadata-change
    // event, which the watcher would pick back up and reprocess forever.
    if existing.as_deref() == Some(final_comment.as_str()) {
        return None;
    }

    finder_comment::set_comment(path, &final_comment).ok()?;
    Some(final_comment)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    fn tags_for(filename: &str) -> Vec<String> {
        let config = Config::default();
        let mut patterns = CompiledPatterns::default();
        patterns.refresh(&config.patterns);
        patterns.detect_tags(filename)
    }

    #[test]
    fn detects_season_episode() {
        assert_eq!(
            tags_for("Gachiakuta.S01E24.FiNAL.MULTi.1080p.WEBRiP.x265-T3KASHi.mkv"),
            vec!["S01E24"]
        );
        assert_eq!(
            tags_for("Gachiakuta.S01E16.MULTi.1080p.WEBRiP.x265-T3KASHi.mkv"),
            vec!["S01E16"]
        );
    }

    #[test]
    fn detects_volume_full_word() {
        assert_eq!(tags_for("Volume 6.cbz"), vec!["Volume 6"]);
        assert_eq!(tags_for("Volume 13.cbz"), vec!["Volume 13"]);
    }

    #[test]
    fn detects_tome_abbreviation() {
        assert_eq!(tags_for("One Piece T01.cbz"), vec!["T01"]);
        assert_eq!(tags_for("One Piece T.01.cbz"), vec!["T.01"]);
        assert_eq!(tags_for("One Piece Vol.05.cbz"), vec!["Vol.05"]);
    }

    #[test]
    fn no_false_positive_on_season_only_pack_name() {
        // A season-pack folder name without an episode number should not match SxxExx.
        assert!(tags_for("Gachiakuta.S01.MULTi.1080p.WEBRiP.x265-T3KASHi.nfo").is_empty());
    }

    #[test]
    fn no_false_positive_on_generic_movie_titles() {
        assert!(tags_for(
            "Barry Lyndon (1975) MULTi VFI 2160p 10bit 4KLight DV HDR BluRay DDP 5.1 x265-QTZ.mkv"
        )
        .is_empty());
        assert!(tags_for(
            "Call.Me.By.Your.Name.2017.MULTi.VFF.BLURAY.2160p.4KLight.DV.HDR.x265.DDP.5.1-CiSCO.mkv"
        )
        .is_empty());
        assert!(tags_for("Chien.51.2025.FRENCH.1080p.WEB.H265-SUPPLY.mkv").is_empty());
    }

    #[test]
    fn extension_filter_rejects_unlisted_extensions() {
        let config = Config::default();
        assert!(extension_allowed(
            Path::new("movie.mkv"),
            &config.extensions
        ));
        assert!(extension_allowed(
            Path::new("movie.MKV"),
            &config.extensions
        ));
        assert!(!extension_allowed(
            Path::new("readme.txt"),
            &config.extensions
        ));
    }

    #[test]
    #[ignore = "requires an interactive macOS session with Finder Automation permission granted; not available on headless CI runners"]
    fn process_file_is_idempotent_once_tagged() {
        let dir = std::env::temp_dir().join(format!("spine_matcher_test_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file_path = dir.join("Show.S01E02.mkv");
        std::fs::write(&file_path, b"fake video content").unwrap();

        let config = Config::default();
        let mut patterns = CompiledPatterns::default();
        patterns.refresh(&config.patterns);

        // First pass: the file gets tagged.
        assert_eq!(
            process_file(&file_path, &config, &patterns),
            Some("S01E02".to_string())
        );
        // Second pass over an already-correctly-tagged file must be a no-op,
        // otherwise a watcher reacting to its own writes would loop forever.
        assert_eq!(process_file(&file_path, &config, &patterns), None);

        std::fs::remove_dir_all(&dir).ok();
    }
}
