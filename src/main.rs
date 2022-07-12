mod client;
mod playlists;
mod redirect_uri;

use anyhow::{Context, Result};
use chrono::prelude::*;
use futures::stream::StreamExt;
use playlists::MonthlyPlaylist;
use rspotify::{prelude::*, ClientResult};
use rspotify_model::{enums::misc::Market, idtypes::PlaylistId, PlayableItem, PlaylistItem};
use std::{collections::hash_map::HashMap, str::FromStr, sync::Arc};
use tokio::sync::Mutex;

//const PLAYLIST_ID: &str = "3jkp8yVGbbIaQ5TOnFEhA9";
const PLAYLIST_ID: &str = "1q9ngMIcrEN08RKSM24Qf0";
const PORT: u16 = 8888;
const CLIENT_ID: &str = "f88fd03782f54480964415eb6fd1a1f8";
const PUBLIC: bool = true;
const FORMAT_STR: &str = "%B %y";
const LOCALE: Locale = Locale::es_CR;

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

    // TODO: uncomment this when going back to own playlists
    //if user.id != playlist.owner.id {
    //    return Err(anyhow!("Authenticated user does not own the playlist provided. You must own the playlist chosen for this program."));
    //}

    type MonthlyHashMap = HashMap<MonthlyPlaylist, (PlaylistId, Vec<PlayableItem>)>;
    let monthly_playlists: Arc<Mutex<MonthlyHashMap>> = Arc::new(Mutex::new(HashMap::new()));

    let today = Utc::today();
    let month_start: DateTime<Utc> = Utc.ymd(today.year(), today.month(), 1).and_hms(0, 0, 0);

    // id needs to be declared here (in the same lifetime as songs) to avoid some lifetime errors
    let id = PlaylistId::from_str(PLAYLIST_ID)?;

    spotify
        .playlist_items(&id, None, Some(&Market::FromToken))
        .for_each_concurrent(None, |p: ClientResult<PlaylistItem>| async {
            if let Ok(p_item) = p {
                //playlists::print_item_info(&p_item);
                if let Some(playable) = p_item.track {
                    // Only local files return None from id(), we just ignore those
                    if playable.id().is_some() {
                        let added_at = p_item.added_at.unwrap();
                        if added_at < month_start {
                            let p_item_monthly =
                                MonthlyPlaylist::new(added_at.year(), added_at.month());

                            let mut monthly_playlists = monthly_playlists.lock().await;
                            if let Some((_, tracks)) = monthly_playlists.get_mut(&p_item_monthly) {
                                tracks.push(playable);
                            } else {
                                let id = playlists::create_playlist(
                                    &spotify,
                                    &user.id,
                                    PUBLIC,
                                    p_item_monthly,
                                    FORMAT_STR,
                                    LOCALE,
                                )
                                .await
                                .expect("Error cloning playlist");
                                monthly_playlists.insert(p_item_monthly, (id, vec![playable]));
                            }
                        };
                    }
                }
            }
        })
        .await;

    //TODO: Create methods to move songs that handles the adding and deleting concurrently and
    //carry it out for all songs in `monthly_playlists` DON'T do one song at a time

    Ok(())
}
