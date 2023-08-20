use std::error::Error;
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use notify::{self, Watcher};

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum AssetsWatcherErrorKind {
    InitializationError,
}

#[derive(Debug)]
pub struct AssetsWatcherError {
    source: Option<Box<dyn Error + 'static>>,
    message: String,
    kind: AssetsWatcherErrorKind,
}

impl AssetsWatcherError {
    pub fn new(
        message: impl AsRef<str>,
        kind: AssetsWatcherErrorKind,
        source: Option<Box<dyn Error + 'static>>,
    ) -> AssetsWatcherError {
        AssetsWatcherError {
            source,
            message: message.as_ref().into(),
            kind,
        }
    }
}

impl fmt::Display for AssetsWatcherError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl Error for AssetsWatcherError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.source.as_deref()
    }
}

pub struct AssetsWatcher {
    watcher: notify::RecommendedWatcher,
    stale_paths: Arc<RwLock<Vec<PathBuf>>>,
}

impl AssetsWatcher {
    pub fn new() -> Result<Self, AssetsWatcherError> {
        fn watcher_func(
            stale_paths: &Arc<RwLock<Vec<PathBuf>>>,
            event: notify::Result<notify::Event>,
        ) {
            match event {
                Ok(notify::Event {
                    kind: notify::EventKind::Modify(notify::event::ModifyKind::Any),
                    paths,
                    ..
                }) => {
                    let mut lock_guard = match stale_paths.write() {
                        Ok(lock_guard) => lock_guard,
                        Err(error) => {
                            panic!(
                                "watcher attempted to write-lock a poisoned lock on \
                                the tracked stale assets. Error: {:?}",
                                error
                            );
                        }
                    };
                    for path in paths {
                        lock_guard.push(path);
                    }
                }
                Err(error) => println!("[STUB] Watcher error for asset manager occurred: {error}"),
                _ => {}
            }
        }

        let stale_paths: Arc<RwLock<Vec<PathBuf>>> = Arc::new(RwLock::new(vec![]));

        let stale_paths_clone = Arc::clone(&stale_paths);
        let watcher = match notify::recommended_watcher(move |event| {
            watcher_func(&stale_paths_clone, event);
        }) {
            Ok(watcher) => watcher,
            Err(error) => {
                return Err(AssetsWatcherError::new(
                    "asset manager watcher error",
                    AssetsWatcherErrorKind::InitializationError,
                    Some(Box::new(error)),
                ));
            }
        };

        Ok(Self {
            watcher,
            stale_paths,
        })
    }

    pub fn add_paths_to_watchlist<S: AsRef<str>>(&mut self, paths: &Vec<S>) {
        for path in paths {
            // Docs of notify-rs does not specify any reason for an error to be returned, so
            // for now, we can confidently use unwrap() in this case.
            self.watcher
                .watch(Path::new(path.as_ref()), notify::RecursiveMode::Recursive)
                .unwrap();
        }
    }

    pub fn get_stale_paths(&self) -> Vec<PathBuf> {
        let lock_guard = match self.stale_paths.read() {
            Ok(lock_guard) => lock_guard,
            Err(error) => {
                // CHANGE THIS TO ERROR INSTEAD OF PANIC.
                panic!(
                    "watcher attempted to read-lock a poisoned lock on the \
                    tracked stale assets. Error: {:?}",
                    error
                );
            }
        };

        let mut paths: Vec<PathBuf> = vec![];
        for path in lock_guard.iter() {
            paths.push(path.clone());
        }

        paths
    }

    pub fn clear_stale_paths(&self) {
        let mut lock_guard = match self.stale_paths.write() {
            Ok(lock_guard) => lock_guard,
            Err(error) => {
                // CHANGE THIS TO ERROR INSTEAD OF PANIC.
                panic!(
                    "watcher attempted to read-lock a poisoned lock on the \
                    tracked stale assets. Error: {:?}",
                    error
                );
            }
        };

        lock_guard.clear();
    }
}
