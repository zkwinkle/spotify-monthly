mod client;
mod playlists;
mod redirect_uri;

use anyhow::{Context, Result};
use chrono::prelude::*;
use futures::join;
use futures::stream::StreamExt;
use playlists::MonthlyPlaylist;
use rspotify::{prelude::*, ClientResult};
use rspotify_model::{enums::misc::Market, idtypes::PlaylistId, PlayableItem, PlaylistItem};
use std::{collections::hash_map::HashMap, iter::zip, str::FromStr, sync::Arc};
use tokio::sync::Mutex;

//const PLAYLIST_ID: &str = "3jkp8yVGbbIaQ5TOnFEhA9";
//cyber-goth
//const PLAYLIST_ID: &str = "1q9ngMIcrEN08RKSM24Qf0";
//bea
const PLAYLIST_ID: &str = "0DDKrqHzWNeIRNZwcxjAqD";
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

    let user = spotify.me();

    let playlist_id = PlaylistId::from_str(PLAYLIST_ID)?;
    let playlist = spotify.playlist(&playlist_id, None, Some(&Market::FromToken));

    let (user, playlist) = join!(user, playlist);
    let (user, playlist) = (user?, playlist?);

    // TODO: uncomment this when going back to own playlists
    //if user.id != playlist.owner.id {
    //    return Err(anyhow!("Authenticated user does not own the playlist provided. You must own the playlist chosen for this program."));
    //}

    type MonthlyHashMap = HashMap<MonthlyPlaylist, (PlaylistId, Vec<PlayableItem>)>;
    let monthly_playlists: Arc<Mutex<MonthlyHashMap>> = Arc::new(Mutex::new(HashMap::new()));

    let today = Utc::today();
    let month_start: DateTime<Utc> = Utc.ymd(today.year(), today.month(), 1).and_hms(0, 0, 0);

    spotify
        .playlist_items(&playlist_id, None, Some(&Market::FromToken))
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

    //TODO: Change this for loop to be a stream using for_each_concurrent? (stream::iter())
    let monthly_playlists = monthly_playlists.lock().await;
    for (month, (month_id, tracks)) in zip(monthly_playlists.keys(), monthly_playlists.values()) {
        playlists::move_songs(&spotify, &playlist_id, month_id, pitem_to_pid(tracks)).await?;
        spotify
            .playlist_unfollow(month_id)
            .await
            .context("Error unfollowing playlist: {}")?;
    }

    Ok(())
}

/// Shorthand to convert a Vec of PlayableItem into an iterator of the item's ids, assumes all
/// PlayableItem have ids by calling unwrap()
fn pitem_to_pid(vec: &[PlayableItem]) -> impl Iterator<Item = &dyn PlayableId> + Clone {
    vec.iter().map(|item| item.id().unwrap())
}
