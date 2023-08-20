use crate::assets::{Asset, AssetError, AssetErrorKind};

use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum AssetManagerErrorKind {
    AssetLoadError,
    AssetLockPoisoned,
    AssetReloadError,
    AssetDestructionError,
    CurrentWorkingDirectoryError,
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
    file_path_to_asset_id_map: HashMap<PathBuf, String>,
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
            file_path_to_asset_id_map: HashMap::new(),
        })
    }

    pub fn load_asset<S: AsRef<str>>(
        &mut self,
        id: S,
        file_path: S,
    ) -> Result<Arc<Mutex<A>>, AssetManagerError> {
        let asset_id = String::from(id.as_ref());

        let mut abs_file_path = match env::current_dir() {
            Ok(path) => path,
            Err(error) => {
                return Err(AssetManagerError::new(
                    "current working directory cannot be used",
                    AssetManagerErrorKind::CurrentWorkingDirectoryError,
                    Some(Box::new(error)),
                ))
            }
        };
        abs_file_path.push(file_path.as_ref());

        match A::new(asset_id.clone(), &abs_file_path) {
            Ok(asset) => {
                self.assets
                    .insert(asset_id.clone(), Arc::new(Mutex::new(asset)));
                self.file_path_to_asset_id_map
                    .insert(abs_file_path, asset_id.clone());

                Ok(Arc::clone(self.assets.get(&asset_id.clone()).unwrap()))
            }
            Err(error) => Err(AssetManagerError::new(
                format!("failed to load asset from \"{}\"", file_path.as_ref()),
                AssetManagerErrorKind::AssetLoadError,
                Some(Box::new(error)),
            )),
        }
    }

    pub fn get_asset<S: AsRef<str>>(&self, id: S) -> Option<Arc<Mutex<A>>> {
        match self.assets.get(id.as_ref().into()) {
            Some(asset_ptr) => Some(Arc::clone(asset_ptr)),
            None => None,
        }
    }

    pub fn reload_asset<S: AsRef<str>>(&mut self, id: S) -> Result<Option<()>, AssetManagerError> {
        match self.assets.get_mut(id.as_ref().into()) {
            Some(ptr) => match ptr.lock() {
                Ok(mut asset) => match asset.reload() {
                    Ok(_) => {}
                    Err(error) => {
                        return Err(AssetManagerError::new(
                            format!("failed to load asset, \"{}\"", id.as_ref()),
                            AssetManagerErrorKind::AssetReloadError,
                            Some(Box::new(error)),
                        ))
                    }
                },
                Err(_) => {
                    return Err(AssetManagerError::new(
                        format!("asset, \"{}\", lock poisoned", id.as_ref()),
                        AssetManagerErrorKind::AssetLockPoisoned,
                        None,
                    ))
                }
            },
            None => return Ok(None),
        }

        match self.run_asset_reload_callbacks(&String::from(id.as_ref())) {
            Ok(_) => return Ok(Some(())),
            Err(error) => {
                return Err(AssetManagerError::new(
                    format!(
                        "unable to call reload callbacks for asset with id, \"{}\"",
                        id.as_ref()
                    ),
                    AssetManagerErrorKind::AssetReloadError,
                    Some(Box::new(error)),
                ))
            }
        };
    }

    pub fn destroy_asset<S: AsRef<str>>(&mut self, id: S) -> Result<Option<()>, AssetManagerError> {
        match self.assets.get_mut(id.as_ref().into()) {
            Some(ptr) => match ptr.lock() {
                Ok(mut asset) => match asset.destroy() {
                    Ok(_) => {
                        self.file_path_to_asset_id_map
                            .remove(asset.get_src_file_path());
                    }
                    Err(error) => {
                        return Err(AssetManagerError::new(
                            format!("failed to destroy asset, \"{}\"", id.as_ref()),
                            AssetManagerErrorKind::AssetReloadError,
                            Some(Box::new(error)),
                        ))
                    }
                },
                Err(err) => {
                    return Err(AssetManagerError::new(
                        format!("asset, \"{}\", lock poisoned", id.as_ref()),
                        AssetManagerErrorKind::AssetLockPoisoned,
                        None,
                    ))
                }
            },
            None => return Ok(None),
        };

        let asset_id = String::from(id.as_ref());
        self.assets.remove(&asset_id);
        self.callbacks.remove(&asset_id);

        Ok(Some(()))
    }

    pub fn is_asset_loaded<S: AsRef<str>>(
        &mut self,
        id: S,
    ) -> Result<Option<bool>, AssetManagerError> {
        match self.assets.get_mut(id.as_ref().into()) {
            Some(ptr) => match ptr.lock() {
                Ok(asset) => Ok(Some(asset.is_loaded())),
                Err(err) => Err(AssetManagerError::new(
                    format!("asset, \"{}\", lock poisoned", id.as_ref()),
                    AssetManagerErrorKind::AssetLockPoisoned,
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

    pub fn reload_assets_by_id<S: AsRef<str>>(
        &mut self,
        ids: &Vec<S>,
    ) -> Result<(), AssetManagerError> {
        for id in ids {
            match self.reload_asset(id) {
                Ok(_) => {}
                Err(error) => {
                    return Err(AssetManagerError::new(
                        format!("failed to reload asset with id, \"{}\"", id.as_ref()),
                        AssetManagerErrorKind::AssetReloadError,
                        Some(Box::new(error)),
                    ))
                }
            }
        }

        Ok(())
    }

    pub fn file_paths_to_asset_ids(&self, paths: &Vec<PathBuf>) -> Vec<String> {
        let mut ids = vec![];
        for path in paths {
            let asset_id = match self.file_path_to_asset_id_map.get(path) {
                Some(id) => id,
                None => {
                    continue;
                }
            };
            ids.push(asset_id.clone());
        }

        ids
    }

    fn run_asset_reload_callbacks(
        &mut self,
        asset_id: &String,
    ) -> Result<Option<()>, AssetManagerError> {
        if let Some(callbacks) = self.callbacks.get(asset_id.as_str()) {
            for func in callbacks {
                func();
            }
        }

        Ok(Some(()))
    }
}
