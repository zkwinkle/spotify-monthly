use anyhow::{Context, Result};
use async_recursion::async_recursion;
use chrono::prelude::*;
use futures::Future;
use num_traits::cast::FromPrimitive;
use reqwest::StatusCode;
use rspotify::{
    http::HttpError,
    model::{
        idtypes::{PlaylistId, UserId},
        PlayableItem, PlaylistItem, PlaylistResult,
    },
    prelude::*,
    AuthCodePkceSpotify, ClientError, ClientResult,
};
use tokio::time::{sleep, Duration};

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

/// Function to call to start recursive adding process
async fn add_recursive<'a, T>(
    spotify: &AuthCodePkceSpotify,
    to_p: &PlaylistId,
    tracks: T,
) -> ClientResult<PlaylistResult>
where
    T: IntoIterator<Item = &'a dyn PlayableId> + Send + Clone + Sync + 'a,
{
    let closure = move || {
        let tracks_clone = tracks.clone(); // Need a longer lived borrow
        async move { spotify.playlist_add_items(to_p, tracks_clone, None).await }
    };
    _recursive_call(closure, 0).await
}

/// Function to call to start recursive removing process
async fn remove_recursive<'a, T>(
    spotify: &AuthCodePkceSpotify,
    from_p: &PlaylistId,
    tracks: T,
) -> ClientResult<PlaylistResult>
where
    T: IntoIterator<Item = &'a dyn PlayableId> + Send + Clone + Sync + 'a,
{
    let closure = move || {
        let tracks_clone = tracks.clone(); // Need a longer lived borrow
        async move {
            spotify
                .playlist_remove_all_occurrences_of_items(
                    from_p,
                    tracks_clone,
                    None,
                )
                .await
        }
    };
    _recursive_call(closure, 0).await
}

#[allow(dead_code)]
pub async fn unfollow_playlist_recursive(
    spotify: &AuthCodePkceSpotify,
    playlist_id: &PlaylistId,
) -> ClientResult<()> {
    let closure = move || {
        let playlist_id_clone = playlist_id.clone(); // Need a longer lived borrow
        async move { spotify.playlist_unfollow(&playlist_id_clone).await }
    };
    _recursive_call(closure, 0).await
}

#[async_recursion]
async fn _recursive_call<F, T, Fut>(f: F, retries: u32) -> ClientResult<T>
where
    F: Fn() -> Fut + Send + Sync, //Pin<Box<dyn Future<Output = ClientResult<T>> + Send>> + Send + Sync,
    Fut: Future<Output = ClientResult<T>> + Send,
    T: Send,
{
    let res = f().await;
    if retries >= MAX_RETRIES {
        return res;
    }

    match res {
        Err(ClientError::Http(box HttpError::StatusCode(ref resp))) => {
            match resp.status() {
                StatusCode::NOT_FOUND
                | StatusCode::BAD_GATEWAY
                | StatusCode::SERVICE_UNAVAILABLE
                | StatusCode::INTERNAL_SERVER_ERROR => {
                    sleep(Duration::from_millis(RETRY_TIME)).await;
                    _recursive_call(f, retries + 1).await
                }
                StatusCode::TOO_MANY_REQUESTS => {
                    sleep(Duration::from_millis(RATE_LIMIT_TIME)).await;
                    _recursive_call(f, retries + 1).await
                }
                _ => res,
            }
        }
        _ => res,
    }
}

pub async fn move_songs<'a, T>(
    spotify: &AuthCodePkceSpotify,
    from_p: &PlaylistId,
    to_p: &PlaylistId,
    tracks: T,
) -> Result<()>
where
    T: IntoIterator<Item = &'a dyn PlayableId> + Send + Sync + Clone + 'a,
{
    let add = add_recursive(spotify, to_p, tracks.clone());
    add.await.context("Adding tracks to new playlist")?;

    // TESTING: For testing purposes uncomment/comment
    let remove = remove_recursive(spotify, from_p, tracks);
    // let remove = futures::future::ready(Result::<()>::Ok(()));
    remove
        .await
        .context("Removing tracks from managed playlist")?;

    Ok(())
}

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
    let date = Local
        .with_ymd_and_hms(
            monthly.year,
            monthly.month.number_from_month(),
            1,
            0,
            0,
            0,
        )
        .unwrap()
        .date_naive();
    let name: &str = &date.format_localized(format_str, lang).to_string();

    Ok(spotify
        .user_playlist_create(user_id, name, Some(public), Some(false), None)
        .await?
        .id)
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
            PlayableItem::Episode(episode) => {
                (&episode.name, episode.show.publisher.clone())
            }
        };
        println!(
            "{} -- {} | Added: {}",
            name,
            from,
            added_at.format("%d of %b %Y, at %H:%M")
        );
    }
}
