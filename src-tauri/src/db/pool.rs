use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use tokio::process::Child;
use tokio::task::AbortHandle;

use crate::db::driver::Driver;
use crate::error::{AppError, AppResult};

/// One live driver per connection id. Arc so commands can clone it out of the
/// lock and then await without holding the mutex.
#[derive(Default)]
pub struct AppState {
    pub conns: Mutex<HashMap<String, Arc<dyn Driver>>>,
    /// In-flight queries by id, so a running query can be cancelled.
    pub running: Mutex<HashMap<String, AbortHandle>>,
    /// SSH tunnel processes by connection id (killed on disconnect via kill_on_drop).
    pub tunnels: Mutex<HashMap<String, Child>>,
}

impl AppState {
    pub fn register_query(&self, query_id: String, handle: AbortHandle) {
        self.running.lock().unwrap().insert(query_id, handle);
    }
    pub fn finish_query(&self, query_id: &str) {
        self.running.lock().unwrap().remove(query_id);
    }
    pub fn cancel(&self, query_id: &str) {
        if let Some(h) = self.running.lock().unwrap().remove(query_id) {
            h.abort();
        }
    }

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

    pub fn insert_tunnel(&self, id: String, child: Child) {
        self.tunnels.lock().unwrap().insert(id, child);
    }

    pub fn remove(&self, id: &str) {
        if let Some(driver) = self.conns.lock().unwrap().remove(id) {
            tokio::spawn(async move { driver.close().await });
        }
        // Dropping the Child tears down the SSH tunnel (kill_on_drop).
        self.tunnels.lock().unwrap().remove(id);
    }
}
