use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::assets::{Asset, AssetError, AssetErrorKind};

pub struct AssetManager<A: Asset> {
    assets: HashMap<String, Arc<Mutex<A>>>,
}

impl<A: Asset> AssetManager<A> {
    pub fn new() -> Self {
        AssetManager {
            assets: HashMap::new(),
        }
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
                Err(err) => Err(AssetError::new(
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
                    Ok(_) => Ok(Some(())),
                    Err(err) => Err(err),
                },
                Err(err) => Err(AssetError::new(
                    format!("asset lock poisoned"),
                    AssetErrorKind::Poisoned,
                    None,
                )),
            },
            None => Ok(None),
        }
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
}
