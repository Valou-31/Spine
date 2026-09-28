use crate::config::Config;
use crate::matcher;
use notify::{Event, EventKind, RecursiveMode, Watcher};
use std::path::PathBuf;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};

pub enum LogMsg {
    Tagged { path: PathBuf, comment: String },
    Info(String),
    Error(String),
}

pub struct WatcherHandle {
    _watcher: notify::RecommendedWatcher,
}

/// Scans the watched folder once, tagging any already-present files that match.
pub fn initial_scan(config: &Config, log_tx: &Sender<LogMsg>) {
    log_tx
        .send(LogMsg::Info(format!(
            "Scan initial de {}...",
            config.watched_folder.display()
        )))
        .ok();
    let mut count = 0;
    for entry in walkdir::WalkDir::new(&config.watched_folder)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if let Some(comment) = matcher::process_file(entry.path(), config) {
            count += 1;
            log_tx
                .send(LogMsg::Tagged {
                    path: entry.path().to_path_buf(),
                    comment,
                })
                .ok();
        }
    }
    log_tx
        .send(LogMsg::Info(format!(
            "Scan initial termine ({} fichier(s) tague(s)).",
            count
        )))
        .ok();
}

/// Starts watching `config.watched_folder` (and subfolders) for changes.
/// The returned handle keeps the watcher alive; drop it to stop watching.
pub fn start(config: Arc<Mutex<Config>>, log_tx: Sender<LogMsg>) -> notify::Result<WatcherHandle> {
    let watch_path = config.lock().unwrap().watched_folder.clone();

    let event_config = config.clone();
    let event_tx = log_tx.clone();
    let mut watcher = notify::recommended_watcher(move |res: notify::Result<Event>| {
        let event = match res {
            Ok(e) => e,
            Err(e) => {
                event_tx.send(LogMsg::Error(e.to_string())).ok();
                return;
            }
        };
        // Only react to events that can introduce/rename a matchable file.
        if !matches!(event.kind, EventKind::Create(_) | EventKind::Modify(_)) {
            return;
        }
        let cfg = event_config.lock().unwrap();
        for path in event.paths.iter() {
            if let Some(comment) = matcher::process_file(path, &cfg) {
                event_tx
                    .send(LogMsg::Tagged {
                        path: path.clone(),
                        comment,
                    })
                    .ok();
            }
        }
    })?;

    watcher.watch(&watch_path, RecursiveMode::Recursive)?;
    log_tx
        .send(LogMsg::Info(format!(
            "Surveillance active sur {}",
            watch_path.display()
        )))
        .ok();

    Ok(WatcherHandle { _watcher: watcher })
}
