use anyhow::{anyhow, Result};
use std::path::{Path, PathBuf};

const REDIRECT_URI: &str = "http://localhost:8888/callback";
const TOKEN_CACHE_FILE: &str = ".monthly_spotify_token_cache.json";

pub fn get_token_path() -> Result<PathBuf> {
    match dirs::cache_dir() {
        Some(cache) => {
            let path = Path::new(&cache);
            let full_path = path.join(TOKEN_CACHE_FILE);

            Ok(full_path)
        }
        None => Err(anyhow!("No cache directory found in OS")),
    }
}
