use anyhow::Result;
use chrono::prelude::*;
use num_traits::cast::FromPrimitive;
use rspotify::{prelude::*, AuthCodePkceSpotify};
use rspotify_model::{
    idtypes::{PlaylistId, UserId},
    PlayableItem, PlaylistItem,
};

use std::str::FromStr;

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

    Ok(PlaylistId::from_str("3jkp8yVGbbIaQ5TOnFEhA9")?)
    //Ok(spotify
    //    .user_playlist_create(user_id, name, Some(public), Some(false), None)
    //    .await?
    //    .id)
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
