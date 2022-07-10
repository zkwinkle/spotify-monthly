mod config;

use anyhow::Result;
use rspotify::{prelude::*, scopes, AuthCodePkceSpotify, Config, Credentials, OAuth};

#[tokio::main]
async fn main() -> Result<()> {
    let creds = Credentials::new_pkce("f88fd03782f54480964415eb6fd1a1f8");

    let oauth = OAuth {
        redirect_uri: "http://localhost:8888/callback".to_string(),
        scopes: scopes!("user-read-recently-played"),
        ..Default::default()
    };

    let config = Config {
        token_cached: true,
        token_refreshing: true,
        cache_path: config::get_token_path()?,
        ..Default::default()
    };

    let mut spotify = AuthCodePkceSpotify::with_config(creds.clone(), oauth.clone(), config);

    // Obtaining the access token
    let url = spotify.get_authorize_url(None).unwrap();
    // This function requires the `cli` feature enabled.
    spotify.prompt_for_token(&url).await.unwrap();

    // Running the requests
    let history = spotify.current_playback(None, None::<Vec<_>>).await;
    println!("Response: {:?}", history);

    // Token refreshing works as well, but only with the one generated in the
    // previous request (they actually expire, unlike the regular code auth
    // flow).
    let prev_token = spotify.token.lock().await.unwrap();
    let spotify = AuthCodePkceSpotify::new(creds, oauth);
    *spotify.token.lock().await.unwrap() = prev_token.clone();
    spotify.refresh_token().await.unwrap();

    // Running the requests again
    let history = spotify.current_playback(None, None::<Vec<_>>).await;
    println!("Response after refreshing token: {:?}", history);

    Ok(())
}
