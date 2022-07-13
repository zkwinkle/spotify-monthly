use anyhow::{Context, Result};
use async_recursion::async_recursion;
use chrono::prelude::*;
use futures::join;
use num_traits::cast::FromPrimitive;
use reqwest::StatusCode;
use rspotify::{prelude::*, AuthCodePkceSpotify, ClientError, ClientResult};
use rspotify_http::HttpError;
use rspotify_model::{
    idtypes::{PlaylistId, UserId},
    PlayableItem, PlaylistItem,
};
use tokio::time::{sleep, Duration};

//use std::str::FromStr;

const RETRY_TIME: u64 = 200; // in ms
const RATE_LIMIT_TIME: u64 = 4000; // in ms
const MAX_RETRIES: u32 = 10;

#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub struct MonthlyPlaylist {
    month: Month,
    year: i32,
}

impl MonthlyPlaylist {
    pub fn new(year: i32, month: u32) -> MonthlyPlaylist {
        MonthlyPlaylist {
            month: Month::from_u32(month).unwrap(),
            year,
        }
    }
}

/// Function to call to start recursive process, just calls add_recursive with a 0 in retries
async fn add_recursive<'a, T>(
    spotify: &AuthCodePkceSpotify,
    to_p: &PlaylistId,
    tracks: T,
) -> ClientResult<rspotify_model::PlaylistResult>
where
    T: IntoIterator<Item = &'a dyn PlayableId> + Send + Clone + 'a,
{
    _add_recursive(spotify, to_p, tracks, 0).await
}

#[async_recursion]
async fn _add_recursive<'a, T>(
    spotify: &AuthCodePkceSpotify,
    to_p: &PlaylistId,
    tracks: T,
    retries: u32,
) -> ClientResult<rspotify_model::PlaylistResult>
where
    T: IntoIterator<Item = &'a dyn PlayableId> + Send + Clone + 'a,
{
    let tracks_clone = tracks.clone(); // Need a longer lived borrow
    let res_add = spotify.playlist_add_items(to_p, tracks_clone, None).await;
    if retries >= MAX_RETRIES {
        return res_add;
    }

    match res_add {
        Err(ClientError::Http(box HttpError::StatusCode(ref resp))) => match resp.status() {
            StatusCode::NOT_FOUND
            | StatusCode::BAD_GATEWAY
            | StatusCode::SERVICE_UNAVAILABLE
            | StatusCode::INTERNAL_SERVER_ERROR => {
                sleep(Duration::from_millis(RETRY_TIME)).await;
                add_recursive(spotify, to_p, tracks).await
            }
            StatusCode::TOO_MANY_REQUESTS => {
                sleep(Duration::from_millis(RATE_LIMIT_TIME)).await;
                add_recursive(spotify, to_p, tracks).await
            }
            _ => res_add,
        },
        _ => res_add,
    }
}

pub async fn move_songs<'a, T>(
    spotify: &AuthCodePkceSpotify,
    from_p: &PlaylistId,
    to_p: &PlaylistId,
    tracks: T,
) -> Result<()>
where
    T: IntoIterator<Item = &'a dyn PlayableId> + Send + Clone + 'a,
{
    // TODO: also handle deleting errors recursively
    // TODO: actually remove the songs once this shid is rdy
    //let remove = spotify.playlist_remove_all_occurrences_of_items(from_p, tracks.clone(), None);
    let remove = futures::future::ready(Result::<()>::Ok(()));

    let add = add_recursive(spotify, to_p, tracks);

    let (res_remove, res_add) = join!(remove, add);

    println!("Checking on: {}", to_p);
    res_remove.context("Removing tracks from managed playlist")?;
    res_add.context("Adding tracks to new playlist")?;

    Ok(())
}
//pub async fn add_song_to_playlist(
//    spotify: &AuthCodePkceSpotify,
//    playlist_id: &PlaylistId,
//    track_id: &dyn PlayableId,
//) -> Result<()> {
//    Ok(())
//}

#[allow(unused_variables)]
pub async fn create_playlist(
    spotify: &AuthCodePkceSpotify,
    user_id: &UserId,
    public: bool,
    monthly: MonthlyPlaylist,
    format_str: &str,
    lang: Locale,
) -> Result<PlaylistId> {
    println!("New monthly: {:?}", monthly);
    let date = Local.ymd(monthly.year as i32, monthly.month.number_from_month(), 1);
    let name: &str = &date.format_localized(format_str, lang).to_string();

    //Ok(PlaylistId::from_str("3jkp8yVGbbIaQ5TOnFEhA9")?)
    Ok(spotify
        .user_playlist_create(user_id, name, Some(public), Some(false), None)
        .await?
        .id)
}

#[allow(dead_code)]
pub async fn unfollow_playlist_recursive(
    spotify: &AuthCodePkceSpotify,
    playlist_id: &PlaylistId,
) -> Result<()> {
    // TODO: Give it same treatment as adding recursively
    if let Err(client_error) = spotify.playlist_unfollow(playlist_id).await {
        eprintln!("Error unfollowing playlist: {:#?}", client_error);
    }
    Ok(())
}

#[allow(dead_code)]
/// Prints {name} -- {artist} | Added: %d of %b %T, at %H:%M
/// for specified playlist item
pub fn print_item_info(p_item: &PlaylistItem) {
    let added_at = p_item.added_at.unwrap();
    if let Some(track) = &(p_item.track) {
        let (name, from): (&str, String) = match track {
            PlayableItem::Track(song) => (&song.name, {
                let artists: String = song
                    .artists
                    .iter()
                    .map(|a| a.name.clone() + ", ")
                    .collect::<String>();
                String::from(&artists[0..artists.len() - 2])
            }),
            PlayableItem::Episode(episode) => (&episode.name, episode.show.publisher.clone()),
        };
        println!(
            "{} -- {} | Added: {}",
            name,
            from,
            added_at.format("%d of %b %Y, at %H:%M")
        );
    }
}
