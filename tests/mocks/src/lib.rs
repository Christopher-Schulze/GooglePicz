use std::collections::HashMap;
use std::sync::Mutex;
use once_cell::sync::Lazy;
use wiremock::MockServer;

pub use wiremock::{Mock, ResponseTemplate, Request};
pub use wiremock::matchers;

static STORE: Lazy<Mutex<HashMap<(String, String), String>>> = Lazy::new(|| Mutex::new(HashMap::new()));

/// Simple in-memory keyring entry used for tests.
#[derive(Clone)]
pub struct Entry {
    service: String,
    user: String,
}

#[derive(Debug)]
pub enum Error {
    NoEntry,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for Error {}

impl Entry {
    pub fn new(service: &str, user: &str) -> Result<Self, Error> {
        Ok(Self { service: service.into(), user: user.into() })
    }

    pub fn set_password(&self, password: &str) -> Result<(), Error> {
        STORE.lock().unwrap().insert((self.service.clone(), self.user.clone()), password.to_string());
        Ok(())
    }

    pub fn get_password(&self) -> Result<String, Error> {
        STORE
            .lock()
            .unwrap()
            .get(&(self.service.clone(), self.user.clone()))
            .cloned()
            .ok_or(Error::NoEntry)
    }
}

/// Start a new wiremock server for API tests.
pub async fn start_mock_server() -> MockServer {
    MockServer::start().await
}

pub fn setup_mock_keyring() {
    STORE.lock().unwrap().clear();
}
