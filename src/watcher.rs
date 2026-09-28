use crate::config::Config;
use crate::matcher::{self, CompiledPatterns};
use notify::{Event, RecursiveMode, Watcher};
use std::path::PathBuf;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};

pub enum LogMsg {
    Tagged { path: PathBuf, comment: String },
    Info(String),
    Error(String),
}

/// Called after every log message so the UI can wake up and repaint - the app
/// requests no periodic repaints on its own, so without this a message would
/// just sit in the channel until some unrelated input event drew a frame.
pub type Waker = Arc<dyn Fn() + Send + Sync>;

pub struct WatcherHandle {
    _watcher: notify::RecommendedWatcher,
}

fn send(log_tx: &Sender<LogMsg>, waker: &Waker, msg: LogMsg) {
    log_tx.send(msg).ok();
    waker();
}

/// Scans the watched folder once, tagging any already-present files that match.
pub fn initial_scan(config: &Config, log_tx: &Sender<LogMsg>, waker: &Waker) {
    let lang = config.language;
    send(
        log_tx,
        waker,
        LogMsg::Info(lang.initial_scan_start(&config.watched_folder.display().to_string())),
    );
    let mut patterns = CompiledPatterns::default();
    patterns.refresh(&config.patterns);

    let mut count = 0;
    for entry in walkdir::WalkDir::new(&config.watched_folder)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if let Some(comment) = matcher::process_file(entry.path(), config, &patterns) {
            count += 1;
            send(
                log_tx,
                waker,
                LogMsg::Tagged {
                    path: entry.path().to_path_buf(),
                    comment,
                },
            );
        }
    }
    send(log_tx, waker, LogMsg::Info(lang.initial_scan_done(count)));
}

/// Starts watching `config.watched_folder` (and subfolders) for changes.
/// The returned handle keeps the watcher alive; drop it to stop watching.
pub fn start(
    config: Arc<Mutex<Config>>,
    log_tx: Sender<LogMsg>,
    waker: Waker,
) -> notify::Result<WatcherHandle> {
    let watch_path = config.lock().unwrap().watched_folder.clone();

    let event_config = config.clone();
    let event_tx = log_tx.clone();
    let event_waker = waker.clone();
    let mut patterns = CompiledPatterns::default();
    let mut watcher = notify::recommended_watcher(move |res: notify::Result<Event>| {
        let event = match res {
            Ok(e) => e,
            Err(e) => {
                send(&event_tx, &event_waker, LogMsg::Error(e.to_string()));
                return;
            }
        };
        // Don't filter by event kind: macOS FSEvents reports some file arrivals
        // (e.g. a download tool renaming a .part file into place) as kinds that
        // don't cleanly map to Create/Modify. process_file() below is cheap to
        // call and already no-ops for anything that isn't a matching file.
        let cfg = event_config.lock().unwrap();
        patterns.refresh(&cfg.patterns);
        for path in event.paths.iter() {
            if let Some(comment) = matcher::process_file(path, &cfg, &patterns) {
                send(
                    &event_tx,
                    &event_waker,
                    LogMsg::Tagged {
                        path: path.clone(),
                        comment,
                    },
                );
            }
        }
    })?;

    watcher.watch(&watch_path, RecursiveMode::Recursive)?;
    let lang = config.lock().unwrap().language;
    send(
        &log_tx,
        &waker,
        LogMsg::Info(lang.watching_active_log(&watch_path.display().to_string())),
    );

    Ok(WatcherHandle { _watcher: watcher })
}
