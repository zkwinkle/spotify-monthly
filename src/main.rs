#![feature(box_patterns)]

mod client;
mod playlists;
mod redirect_uri;

use anyhow::{anyhow, Context, Result};
use chrono::prelude::*;
use futures::{join, prelude::*};
use playlists::MonthlyPlaylist;
use rspotify::{
    model::{
        enums::misc::Market, idtypes::PlaylistId, PlayableItem, PlaylistItem,
        PrivateUser,
    },
    prelude::*,
    AuthCodePkceSpotify, ClientResult,
};
use std::{collections::hash_map::HashMap, iter::zip, str::FromStr, sync::Arc};
use tokio::sync::Mutex;

type MonthlyHashMap = HashMap<MonthlyPlaylist, (PlaylistId, Vec<PlayableItem>)>;

const PLAYLIST_ID: &str = "3jkp8yVGbbIaQ5TOnFEhA9";
const PORT: u16 = 8888;
const CLIENT_ID: &str = "f88fd03782f54480964415eb6fd1a1f8";
const PUBLIC: bool = true;
const FORMAT_STR: &str = "%B %y";
const LOCALE: Locale = Locale::es_CR;
const MONTHS_BUFFER: u32 = 1;

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
    let playlist =
        spotify.playlist(&playlist_id, None, Some(&Market::FromToken));

    let (user, playlist) = join!(user, playlist);
    let (user, playlist) = (user?, playlist?);

    if user.id != playlist.owner.id {
        return Err(anyhow!("Authenticated user does not own the playlist provided. You must own the playlist chosen for this program."));
    }

    let monthly_playlists: Arc<Mutex<MonthlyHashMap>> =
        Arc::new(Mutex::new(HashMap::new()));

    let today = Utc::now();
    let month_start: DateTime<Utc> = Utc
        .with_ymd_and_hms(today.year(), today.month(), 1, 0, 0, 0)
        .unwrap();
    let month_start = sub_months(month_start, MONTHS_BUFFER);
    println!("Month start: {:?}", month_start);

    spotify
        .playlist_items(&playlist_id, None, Some(&Market::FromToken))
        // in series because async part gets locked anyways
        .for_each(|p: ClientResult<PlaylistItem>| {
            create_monthly_playlists(
                p,
                &spotify,
                monthly_playlists.clone(),
                month_start,
                &user,
            )
        })
        .await;

    let monthly_playlists = monthly_playlists.lock().await;
    stream::iter(
        zip(monthly_playlists.keys(), monthly_playlists.values())
            .map(|z| -> Result<_> { Ok(z) }),
    )
    .try_for_each_concurrent(None, |(month, (month_id, tracks))| {
        // Have to move month and tracks inside of async, and reference spotify and playlist_id
        let spotify = &spotify;
        let playlist_id = &playlist_id;
        async move {
            println!("Moving songs to month: {:?} -- {}", month, month_id);
            playlists::move_songs(
                spotify,
                playlist_id,
                month_id,
                pitem_to_pid(tracks),
            )
            .await?;
            // TESTING: For testing purposes uncomment
            // playlists::unfollow_playlist_recursive(spotify, month_id)
            //     .await
            //     .context(format!("Unfollowing playlist: {:?}", &month))?;
            Ok(())
        }
    })
    .await?;

    Ok(())
}

// This function makes the decision of which playlists to create and which
// songs to add to said playlists
async fn create_monthly_playlists(
    p: ClientResult<PlaylistItem>,
    spotify: &AuthCodePkceSpotify,
    monthly_playlists: Arc<Mutex<MonthlyHashMap>>,
    month_start: DateTime<Utc>,
    user: &PrivateUser,
) {
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
                    if let Some((_, tracks)) =
                        monthly_playlists.get_mut(&p_item_monthly)
                    {
                        tracks.push(playable);
                    } else {
                        let id = playlists::create_playlist(
                            spotify,
                            &user.id,
                            PUBLIC,
                            p_item_monthly,
                            FORMAT_STR,
                            LOCALE,
                        )
                        .await
                        .expect("Error cloning playlist");
                        monthly_playlists
                            .insert(p_item_monthly, (id, vec![playable]));
                    }
                };
            }
        }
    }
}

fn sub_months<Tz: TimeZone>(date: DateTime<Tz>, sub: u32) -> DateTime<Tz> {
    if (sub % 12) < date.month() {
        date.with_year(
            date.year() - (TryInto::<i32>::try_into(sub / 12).unwrap()),
        )
        .unwrap()
        .with_month(date.month() - (sub % 12))
        .unwrap()
    } else {
        date.with_year(
            date.year() - (TryInto::<i32>::try_into((sub / 12) + 1).unwrap()),
        )
        .unwrap()
        .with_month(date.month() + (12 - (sub % 12)))
        .unwrap()
    }
}

/// Shorthand to convert a Vec of PlayableItem into an iterator of the item's ids, assumes all
/// PlayableItem have ids by calling unwrap()
fn pitem_to_pid(
    vec: &[PlayableItem],
) -> impl Iterator<Item = &dyn PlayableId> + Clone {
    vec.iter().map(|item| item.id().unwrap())
}
