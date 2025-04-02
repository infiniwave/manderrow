use std::path::PathBuf;
use std::sync::OnceLock;

use anyhow::{anyhow, Context as _, Result};

use crate::{identifier, product_name};

static HOME_DIR: OnceLock<PathBuf> = OnceLock::new();
static CACHE_DIR: OnceLock<PathBuf> = OnceLock::new();
static CONFIG_DIR: OnceLock<PathBuf> = OnceLock::new();
static LOCAL_DATA_DIR: OnceLock<PathBuf> = OnceLock::new();
static RUNTIME_DIR: OnceLock<PathBuf> = OnceLock::new();

pub fn init() -> Result<()> {
    HOME_DIR
        .set(dirs::home_dir().context("Unable to determine home directory")?)
        .map_err(|_| anyhow!("Already set"))?;
    CACHE_DIR
        .set({
            let mut p = dirs::cache_dir().context("Unable to determine cache directory")?;
            p.push(identifier());
            if cfg!(windows) {
                p.push("cache");
            }
            p
        })
        .map_err(|_| anyhow!("Already set"))?;
    CONFIG_DIR
        .set({
            let mut p = dirs::config_dir().context("Unable to determine config directory")?;
            p.push(product_name());
            p
        })
        .map_err(|_| anyhow!("Already set"))?;
    LOCAL_DATA_DIR
        .set({
            let mut p =
                dirs::data_local_dir().context("Unable to determine local data directory")?;
            // TODO: replace with product_name()
            p.push(identifier());
            p
        })
        .map_err(|_| anyhow!("Already set"))?;
    RUNTIME_DIR
        .set({
            let mut p = dirs::runtime_dir().unwrap_or_else(|| {
                let mut p = std::env::temp_dir();
                p.push("runtime");
                p
            });
            p.push(identifier());
            p
        })
        .map_err(|_| anyhow!("Already set"))?;

    assert!(home_dir().is_absolute());
    assert!(cache_dir().is_absolute());
    assert!(config_dir().is_absolute());
    assert!(local_data_dir().is_absolute());
    assert!(runtime_dir().is_absolute());

    std::fs::create_dir_all(cache_dir())?;
    std::fs::create_dir_all(local_data_dir())?;
    std::fs::create_dir_all(runtime_dir())?;

    Ok(())
}

pub fn home_dir() -> &'static PathBuf {
    HOME_DIR.get().unwrap()
}

pub fn cache_dir() -> &'static PathBuf {
    CACHE_DIR.get().unwrap()
}

pub fn config_dir() -> &'static PathBuf {
    CONFIG_DIR.get().unwrap()
}

pub fn local_data_dir() -> &'static PathBuf {
    LOCAL_DATA_DIR.get().unwrap()
}

pub fn runtime_dir() -> &'static PathBuf {
    RUNTIME_DIR.get().unwrap()
}
