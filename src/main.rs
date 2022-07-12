mod client;
mod redirect_uri;

use anyhow::{anyhow, Context, Result};
use futures::stream::StreamExt;
use rspotify::{prelude::*, ClientResult};
use rspotify_model::{enums::misc::Market, idtypes::PlaylistId, PlayableItem, PlaylistItem};
use std::str::FromStr;

const PLAYLIST_ID: &str = "3jkp8yVGbbIaQ5TOnFEhA9";
const PORT: u16 = 8888;
const CLIENT_ID: &str = "f88fd03782f54480964415eb6fd1a1f8";

#[tokio::main]
async fn main() -> Result<()> {
    let mut spotify = client::get_client(CLIENT_ID, PORT)?;

    if let Err(error) = client::get_token_auto(&mut spotify, PORT).await {
        eprintln!("Authentication failed with error: {}", error);
        eprintln!("Wiping cache and attempting re-authentication.");
        client::remove_cache()
            .context("Failed to wipe authentication cache while attempting to re-authenticate.")?;
        client::get_token_auto(&mut spotify, PORT).await?
    }

    let user = spotify.me().await?;

    let playlist = spotify
        .playlist(
            &PlaylistId::from_str(PLAYLIST_ID)?,
            None,
            Some(&Market::FromToken),
        )
        .await?;

    if user.id != playlist.owner.id {
        return Err(anyhow!("Authenticated user does not own the playlist provided. You must own the playlist chosen for this program."));
    }

    // id needs to be declared here (in the same lifetime as songs) to avoid some lifetime errors
    let id = PlaylistId::from_str(PLAYLIST_ID)?;
    let songs = spotify
        .playlist_items(&id, None, Some(&Market::FromToken))
        .for_each_concurrent(None, |p: ClientResult<PlaylistItem>| async {
            if let Ok(p_item) = p {
                let added_at = p_item.added_at.unwrap();
                if let Some(track) = p_item.track {
                    let (name, from) = match track {
                        PlayableItem::Track(song) => (song.name, {
                            let artists: String = song
                                .artists
                                .iter()
                                .map(|a| a.name.clone() + ", ")
                                .collect::<String>();
                            String::from(&artists[0..artists.len() - 2])
                        }),
                        PlayableItem::Episode(episode) => (episode.name, episode.show.publisher),
                    };
                    println!(
                        "{} -- {} | Added: {}",
                        name,
                        from,
                        added_at.format("%d of %b %Y, at %H:%M")
                    );
                }
            }
        });

    songs.await;

    Ok(())
}
