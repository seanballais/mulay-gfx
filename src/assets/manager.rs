use crate::assets::{Asset, AssetError, AssetErrorKind};

use notify::{self, Watcher};

use std::collections::HashMap;
use std::error::Error;
use std::fmt;
use std::path::Path;
use std::sync::{Arc, Mutex, RwLock};

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum AssetManagerErrorKind {
    WatcherError,
    WatcherNotInitializedError,
    AssetReloadError
}

#[derive(Debug)]
pub struct AssetManagerError {
    source: Option<Box<dyn Error + 'static>>,
    message: String,
    kind: AssetManagerErrorKind,
}

impl AssetManagerError {
    pub fn new(
        message: impl AsRef<str>,
        kind: AssetManagerErrorKind,
        source: Option<Box<dyn Error + 'static>>,
    ) -> AssetManagerError {
        AssetManagerError {
            source,
            message: message.as_ref().into(),
            kind,
        }
    }
}

impl fmt::Display for AssetManagerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl Error for AssetManagerError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.source.as_deref()
    }
}

pub struct AssetManager<A: Asset> {
    assets: HashMap<String, Arc<Mutex<A>>>,
    callbacks: HashMap<String, Vec<fn()>>,

    // These help with watcher so that we don't need any mutex locks/unlocks.
    asset_file_paths: Vec<String>,
    file_path_to_asset_id_map: HashMap<String, String>,
}

impl<A: Asset> Drop for AssetManager<A> {
    fn drop(&mut self) {
        // TODO: Implement drop for asset managers.
    }
}

impl<A: Asset> AssetManager<A> {
    pub fn new() -> Result<Self, AssetManagerError> {
        Ok(Self {
            assets: HashMap::new(),
            callbacks: HashMap::new(),
            asset_watcher: None,
            stale_asset_paths: Arc::new(RwLock::new(vec![])),
            asset_file_paths: vec![],
            file_path_to_asset_id_map: HashMap::new(),
        })
    }

    pub fn load_asset<S: AsRef<str>>(
        &mut self,
        id: S,
        file_path: S,
    ) -> Result<Arc<Mutex<A>>, AssetError> {
        let asset_id = String::from(id.as_ref());
        let asset_file_path = String::from(file_path.as_ref());
        match A::new(asset_id.clone(), asset_file_path.clone()) {
            Ok(asset) => {
                self.assets
                    .insert(asset_id.clone(), Arc::new(Mutex::new(asset)));
                self.asset_file_paths.push(asset_file_path.clone());
                self.file_path_to_asset_id_map
                    .insert(asset_file_path.clone(), asset_id.clone());

                match &mut self.asset_watcher {
                    Some(watcher) => {
                        watcher.watch(Path::new(&asset_file_path), notify::RecursiveMode::Recursive).unwrap();
                    },
                    None => {}
                }

                Ok(Arc::clone(self.assets.get(&asset_id.clone()).unwrap()))
            }
            Err(err) => Err(err),
        }
    }

    pub fn get_asset<S: AsRef<str>>(&self, id: S) -> Option<Arc<Mutex<A>>> {
        match self.assets.get(id.as_ref().into()) {
            Some(asset_ptr) => Some(Arc::clone(asset_ptr)),
            None => None,
        }
    }

    pub fn reload_asset<S: AsRef<str>>(&mut self, id: S) -> Result<Option<()>, AssetError> {
        match self.assets.get_mut(id.as_ref().into()) {
            Some(ptr) => match ptr.lock() {
                Ok(mut asset) => match asset.reload() {
                    Ok(_) => Ok(Some(())),
                    Err(err) => Err(err),
                },
                Err(_) => Err(AssetError::new(
                    format!("asset lock poisoned"),
                    AssetErrorKind::Poisoned,
                    None,
                )),
            },
            None => Ok(None),
        }
    }

    pub fn destroy_asset<S: AsRef<str>>(&mut self, id: S) -> Result<Option<()>, AssetError> {
        match self.assets.get_mut(id.as_ref().into()) {
            Some(ptr) => match ptr.lock() {
                Ok(mut asset) => match asset.destroy() {
                    Ok(_) => {}
                    Err(err) => {
                        return Err(err);
                    }
                },
                Err(err) => {
                    return Err(AssetError::new(
                        "asset lock poisoned",
                        AssetErrorKind::Poisoned,
                        None,
                    ));
                }
            },
            None => {
                return Ok(None);
            }
        }

        let asset_id = String::from(id.as_ref());
        self.assets.remove(&asset_id);
        self.callbacks.remove(&asset_id);
        self.asset_file_paths.retain(|path| path != &asset_id);
        self.file_path_to_asset_id_map.remove(&asset_id);

        Ok(Some(()))
    }

    pub fn is_asset_loaded<S: AsRef<str>>(&mut self, id: S) -> Result<Option<bool>, AssetError> {
        match self.assets.get_mut(id.as_ref().into()) {
            Some(ptr) => match ptr.lock() {
                Ok(asset) => Ok(Some(asset.is_loaded())),
                Err(err) => Err(AssetError::new(
                    format!("asset lock poisoned"),
                    AssetErrorKind::Poisoned,
                    None,
                )),
            },
            None => Ok(None),
        }
    }

    pub fn register_asset_reload_callback<S: AsRef<str>>(
        &mut self,
        target_asset_id: S,
        callback: fn(),
    ) {
        match self.callbacks.get_mut(target_asset_id.as_ref().into()) {
            Some(callbacks) => callbacks.push(callback),
            None => {
                self.callbacks
                    .insert(target_asset_id.as_ref().into(), vec![callback]);
            }
        };
    }

    pub fn start_watcher(&mut self) -> Result<(), AssetManagerError> {
        fn watcher_func(
            stale_asset_paths: &Arc<RwLock<Vec<String>>>,
            event: notify::Result<notify::Event>,
        ) {
            match event {
                Ok(notify::Event {
                    kind: notify::EventKind::Modify(notify::event::ModifyKind::Any),
                    paths,
                    ..
                }) => {
                    let mut lock_guard = match stale_asset_paths.write() {
                        Ok(lock_guard) => lock_guard,
                        Err(error) => {
                            panic!(
                                "watcher for an asset manager \
                                attempted to write-lock a poisoned lock on the \
                                tracked stale assets. Error: {:?}",
                                error
                            );
                        }
                    };
                    for path in paths {
                        let path_string = String::from(path.into_os_string().to_string_lossy());
                        lock_guard.push(path_string);
                    }
                }
                Err(error) => println!("[STUB] Watcher error for asset manager occurred: {error}"),
                _ => {}
            }
        }

        if self.asset_watcher.is_none() {
            let stale_asset_paths = Arc::clone(&self.stale_asset_paths);
            let watcher = match notify::recommended_watcher(move |event| {
                watcher_func(&stale_asset_paths, event);
            }) {
                Ok(watcher) => watcher,
                Err(error) => {
                    return Err(AssetManagerError::new(
                        "asset manager watcher error",
                        AssetManagerErrorKind::WatcherError,
                        Some(Box::new(error)),
                    ));
                }
            };

            self.asset_watcher = Some(watcher);
        }

        match &mut self.asset_watcher {
            Some(watcher) => {
                for path in &self.asset_file_paths {
                    // Docs of notify-rs does not specify any reason for an error to be returned, so
                    // for now, we can confidently use unwrap() in this case.
                    watcher.watch(Path::new(path), notify::RecursiveMode::Recursive).unwrap();
                }
            },
            None => {
                return Err(AssetManagerError::new(
                    "watcher not yet started",
                    AssetManagerErrorKind::WatcherNotInitializedError,
                    None,
                ));
            }
        }

        Ok(())
    }

    pub fn watch_for_changes(&mut self) -> Result<(), AssetManagerError> {
        let lock_guard = match self.stale_asset_paths.read() {
            Ok(lock_guard) => lock_guard,
            Err(error) => {
                // CHANGE THIS TO ERROR INSTEAD OF PANIC.
                panic!(
                    "watcher for an asset manager \
                    attempted to read-lock a poisoned lock on the \
                    tracked stale assets. Error: {:?}",
                    error
                );
            }
        };
        for asset_path in lock_guard.iter() {
            let asset_id = match self.file_path_to_asset_id_map.get(asset_path) {
                Some(id) => id,
                None => continue
            };
            match self.run_asset_reload_callbacks(asset_id) {
                Ok(_) => {},
                Err(error) => {
                    return Err(AssetManagerError::new(
                        "unable to reload asset",
                        AssetManagerErrorKind::AssetReloadError,
                        Some(Box::new(error)),
                    ));
                }
            }
        }
    
        Ok(())
    }

    fn run_asset_reload_callbacks(&mut self, asset_id: &String) -> Result<Option<()>, AssetError> {
        match self.reload_asset(asset_id.as_str()) {
            Ok(_) => {}
            Err(error) => return Err(error),
        };

        if let Some(callbacks) = self.callbacks.get(asset_id.as_str()) {
            for func in callbacks {
                func();
            }
        }

        Ok(Some(()))
    }
}

pub fn watch_for_asset_changes()
