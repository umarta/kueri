use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::db::driver::Driver;
use crate::error::{AppError, AppResult};

/// One live driver per connection id. Arc so commands can clone it out of the
/// lock and then await without holding the mutex.
#[derive(Default)]
pub struct AppState {
    pub conns: Mutex<HashMap<String, Arc<dyn Driver>>>,
}

impl AppState {
    pub fn get(&self, id: &str) -> AppResult<Arc<dyn Driver>> {
        self.conns
            .lock()
            .unwrap()
            .get(id)
            .cloned()
            .ok_or_else(|| AppError::ConnectionNotFound(id.to_string()))
    }

    pub fn insert(&self, id: String, driver: Arc<dyn Driver>) {
        self.conns.lock().unwrap().insert(id, driver);
    }

    pub fn remove(&self, id: &str) {
        if let Some(driver) = self.conns.lock().unwrap().remove(id) {
            tokio::spawn(async move { driver.close().await });
        }
    }
}
