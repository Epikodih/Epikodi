pub struct AppState {
    pub host: Option<String>,
}

impl AppState {
    pub fn new(host: Option<String>) -> Self {
        Self { host }
    }
}