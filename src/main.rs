mod client;
mod playlists;
mod redirect_uri;

use anyhow::{Context, Result};
use chrono::prelude::*;
use futures::stream::StreamExt;
use playlists::MonthlyPlaylist;
use rspotify::{prelude::*, ClientResult};
use rspotify_model::{enums::misc::Market, idtypes::PlaylistId, PlaylistItem};
use std::{
    collections::hash_map::{Entry, HashMap},
    str::FromStr,
    sync::Arc,
};
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

    let created_playlists: Arc<Mutex<HashMap<MonthlyPlaylist, PlaylistId>>> =
        Arc::new(Mutex::new(HashMap::new()));

    let today = Utc::today();
    let month_start: DateTime<Utc> = Utc.ymd(today.year(), today.month(), 1).and_hms(0, 0, 0);

    // id needs to be declared here (in the same lifetime as songs) to avoid some lifetime errors
    let id = PlaylistId::from_str(PLAYLIST_ID)?;

    let songs = spotify
        .playlist_items(&id, None, Some(&Market::FromToken))
        .for_each_concurrent(None, |p: ClientResult<PlaylistItem>| {
            //let created_playlists = created_playlists.clone();
            async {
                if let Ok(p_item) = p {
                    //playlists::print_item_info(&p_item);

                    let added_at = p_item.added_at.unwrap();

                    if added_at < month_start {
                        let p_item_monthly =
                            MonthlyPlaylist::new(added_at.year(), added_at.month());

                        {
                            let mut created_playlists = created_playlists.lock().await;
                            // Allow because doing with Entry would be the same but less readable
                            #[allow(clippy::map_entry)]
                            if !created_playlists.contains_key(&p_item_monthly) {
                                created_playlists.insert(
                                    p_item_monthly,
                                    playlists::create_playlist(
                                        &spotify,
                                        &user.id,
                                        PUBLIC,
                                        p_item_monthly,
                                        FORMAT_STR,
                                        LOCALE,
                                    )
                                    .await
                                    .expect("Error cloning playlist"),
                                );
                            }
                        }
                    }
                }
            }
        });

    songs.await;

    Ok(())
}
