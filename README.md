# Spotify Monthly
Automatically manage monthly playlists!

## Setup

## Arguments

- playlist_id: Id of the playlist (found in the url)
- port: Port of client that retrieves OAuth token (defaults to `8888`)
- client_id: Spotify client ID
- public: Whether the monthly playlists should be public (defaults to `true`)
- format_str: Format string for the monthly playlist names (Based on [chrono's strftime](https://docs.rs/chrono/latest/chrono/format/strftime/index.html), defaults to `"%B %y"` so for example "mayo 23" for May 2023 using es_CR locale);
- locale: The locale is used to format the playlist names. Defaults to `es_CR` (Costa Rica Spanish). [Pick one from pure_rust_locales.](https://docs.rs/pure-rust-locales/latest/pure_rust_locales/)
- months_buffer: The amount of months back to keep in the managed playlist. For example for `0` it will keep no months in the managed playlist except from the current one. With `1` it will keep the current and past month in the managed playlist. With `12` you keep a full year before moving the songs to their monthly playlists, so on and so forth. Defaults to 1.

## TODO
- CLAP
- Add dry run flag
- Docs
- logging of errors (if i plan on running it automatically)
- Instructions
- PR to fix playlist_remove_all_occurrences_of_items() docs (add instead of remove)
