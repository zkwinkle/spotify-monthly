mod client;

use anyhow::{Context, Result};
use futures::stream::StreamExt;
use rspotify::prelude::*;
//use std::borrow::Borrow;

const PLAYLIST_ID: &str = "3jkp8yVGbbIaQ5TOnFEhA9";

#[tokio::main]
async fn main() -> Result<()> {
    let mut spotify = client::get_client()?;

    // Obtaining the access token
    let url = spotify.get_authorize_url(None)?;

    if let Err(error) = spotify.prompt_for_token(&url).await {
        eprintln!("Authentication failed with error: {}", error);
        eprintln!("Wiping cache and attempting re-authentication.");
        client::remove_cache()
            .context("Failed to wipe authentication cache while attempting to re-authenticate.")?;
        spotify.prompt_for_token(&url).await?
    }

    let user_playlists = spotify.current_user_playlists();
    let playlist = user_playlists
        .filter(|x| {
            futures::future::ready(match x {
                Ok(playlist) => playlist.id.as_ref() == PLAYLIST_ID,
                _ => false,
            })
        })
        .next()
        .await;

    match playlist {
        Some(p) => println!("Found playlist!!:\n{:#?}", p),
        None => println!("Couldn't find a playlist with the given ID"),
    }

    Ok(())
}
