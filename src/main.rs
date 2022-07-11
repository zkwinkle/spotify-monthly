mod client;

use anyhow::{Context, Result};
use futures::stream::StreamExt;
use rspotify::prelude::*;
use rspotify_model::{enums::misc::Market, idtypes::PlaylistId};
use std::str::FromStr;

//la mía
const PLAYLIST_ID: &str = "3jkp8yVGbbIaQ5TOnFEhA9";
//
//la de spotify
//const PLAYLIST_ID: &str = "37i9dQZF1DX0XUsuxWHRQd";

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

    let user = spotify.me().await?;
    let playlist = spotify
        .playlist(
            &PlaylistId::from_str(PLAYLIST_ID)?,
            None,
            Some(&Market::FromToken),
        )
        .await?;

    println!("playlist owner: {:#?}", playlist.owner);
    println!("self: {:#?}", user);
    println!("comparison: {:#?}", user.id == playlist.owner.id);
    println!("playlist: {:#?}", playlist);

    Ok(())
}
