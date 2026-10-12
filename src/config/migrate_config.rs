use serde::Deserialize;

use crate::config::{DatabaseConfig, load};

#[derive(Deserialize)]
pub struct MigrationConfig {
    pub database: DatabaseConfig,
}

impl MigrationConfig {
    pub fn load() -> anyhow::Result<Self> {
        load()
    }
}
