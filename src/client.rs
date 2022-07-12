use crate::redirect_uri;
use anyhow::{anyhow, Result};
use rspotify::{prelude::*, scopes, AuthCodePkceSpotify, Config, Credentials, OAuth};
use std::{
    fs,
    path::{Path, PathBuf},
};

const TOKEN_CACHE_FILE: &str = ".monthly_spotify_token_cache.json";

pub fn get_client(client_id: &str, port: u16) -> Result<AuthCodePkceSpotify> {
    let creds = Credentials::new_pkce(client_id);

    let scopes = scopes!("playlist-modify-private", "playlist-modify-public");

    let oauth = OAuth {
        redirect_uri: format!("http://localhost:{}/callback", port),
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

pub async fn prompt_token_auto(
    spotify: &mut AuthCodePkceSpotify,
    url: &str,
    port: u16,
) -> Result<()> {
    match redirect_uri::redirect_uri_web_server(spotify, url, port) {
        Ok(code) => spotify.request_token(&code).await.map_err(|e| anyhow!(e)),
        Err(e) => {
            eprintln!("{}", e);
            println!("Starting webserver failed. Continuing with manual authentication");
            let code = spotify.get_code_from_user(url)?;
            spotify.request_token(&code).await.map_err(|e| anyhow!(e))
        }
    }
}

/// get token automatically with local webserver
pub async fn get_token_auto(spotify: &mut AuthCodePkceSpotify, port: u16) -> Result<()> {
    let url = spotify.get_authorize_url(None)?;

    match spotify.read_token_cache(true).await {
        Ok(Some(new_token)) => {
            let expired = new_token.is_expired();

            // Load token into client regardless of whether it's expired o
            // not, since it will be refreshed later anyway.
            *spotify.get_token().lock().await.unwrap() = Some(new_token);

            if expired {
                // Ensure that we actually got a token from the refetch
                match spotify.refetch_token().await? {
                    Some(refreshed_token) => {
                        log::info!("Successfully refreshed expired token from token cache");
                        *spotify.get_token().lock().await.unwrap() = Some(refreshed_token);
                    }
                    // If not, prompt the user for it
                    None => {
                        log::info!("Unable to refresh expired token from token cache");
                        prompt_token_auto(spotify, &url, port).await?
                    }
                }
            }
        }
        // Otherwise following the usual procedure to get the token.
        _ => prompt_token_auto(spotify, &url, port).await?,
    }

    spotify.write_token_cache().await.map_err(|e| anyhow!(e))
}
