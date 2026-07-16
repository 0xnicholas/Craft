use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use notify::{RecommendedWatcher, RecursiveMode, Watcher as NotifyWatcher};

#[derive(Debug, Clone)]
pub enum WatcherEvent {
    Changed(PathBuf),
    Removed(PathBuf),
}

pub struct Watcher {
    _inner: RecommendedWatcher,
    pub receiver: mpsc::Receiver<WatcherEvent>,
    debounce: Duration,
}

impl Watcher {
    pub fn new(root: &Path) -> Result<Self, notify::Error> {
        let (tx, rx) = mpsc::channel();
        let mut inner = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
            if let Ok(ev) = res {
                for path in ev.paths {
                    use notify::event::EventKind;
                    let kind = match ev.kind {
                        EventKind::Remove(_) => WatcherEvent::Removed(path),
                        _ => WatcherEvent::Changed(path),
                    };
                    let _ = tx.send(kind);
                }
            }
        })?;
        inner.watch(root, RecursiveMode::Recursive)?;
        Ok(Self {
            _inner: inner,
            receiver: rx,
            debounce: Duration::from_millis(100),
        })
    }

    pub fn drain_debounced(&self) -> Vec<WatcherEvent> {
        let mut by_path: std::collections::HashMap<PathBuf, WatcherEvent> =
            std::collections::HashMap::new();
        let deadline = Instant::now() + self.debounce;
        loop {
            let now = Instant::now();
            if now >= deadline {
                break;
            }
            let timeout = deadline - now;
            match self.receiver.recv_timeout(timeout) {
                Ok(ev) => {
                    let path = match &ev {
                        WatcherEvent::Changed(p) | WatcherEvent::Removed(p) => p.clone(),
                    };
                    by_path.insert(path, ev);
                }
                Err(mpsc::RecvTimeoutError::Timeout) => break,
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            }
        }
        by_path.into_values().collect()
    }
}
