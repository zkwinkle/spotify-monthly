use anyhow::{anyhow, Result};
use rspotify::{scopes, AuthCodePkceSpotify, Config, Credentials, OAuth};
use std::{
    fs,
    path::{Path, PathBuf},
};

const REDIRECT_URI: &str = "http://localhost:8888/callback";
const TOKEN_CACHE_FILE: &str = ".monthly_spotify_token_cache.json";
const CLIENT_ID: &str = "f88fd03782f54480964415eb6fd1a1f8";

pub fn get_client() -> Result<AuthCodePkceSpotify> {
    let creds = Credentials::new_pkce(CLIENT_ID);

    let scopes = scopes!("user-read-playback-state");

    let oauth = OAuth {
        redirect_uri: REDIRECT_URI.to_string(),
        scopes,
        ..Default::default()
    };

    let config = Config {
        token_cached: true,
        token_refreshing: true,
        cache_path: get_token_path()?,
        ..Default::default()
    };

    Ok(AuthCodePkceSpotify::with_config(creds, oauth, config))
}

fn get_token_path() -> Result<PathBuf> {
    match dirs::cache_dir() {
        Some(cache) => {
            let path = Path::new(&cache);
            let full_path = path.join(TOKEN_CACHE_FILE);

            Ok(full_path)
        }
        None => Err(anyhow!("No cache directory found in OS.")),
    }
}

pub fn remove_cache() -> Result<()> {
    let cache_file = get_token_path()?;
    if cache_file.exists() {
        fs::remove_file(cache_file)?;
        Ok(())
    } else {
        Err(anyhow!(
            "Tried removing non-existent auth token cache file."
        ))
    }
}
