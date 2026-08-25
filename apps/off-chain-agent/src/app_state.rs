use crate::spacetime::{self, SpacetimeConnection};
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub spacetime_conn: Option<Arc<SpacetimeConnection>>,
}

impl AppState {
    pub async fn new() -> Self {
        AppState {
            spacetime_conn: match spacetime::init_spacetimedb_connection().await {
                Ok(conn) => Some(conn),
                Err(e) => {
                    log::error!("Failed to connect to SpacetimeDB: {e}.");
                    None
                }
            },
        }
    }
}
