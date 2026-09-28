use anyhow::{Context, Result, bail};
use async_ffi::async_ffi;
use ferrispot::{
    model::{playback::PlayingType, track::FullTrack},
    prelude::*,
};
use terminal_link::Link;
use tokio::runtime::Handle;

use crate::{LYRICS, SPOTIFY};

#[async_ffi(?Send)]
#[unsafe(no_mangle)]
#[allow(clippy::unnecessary_wraps)]
#[allow(clippy::needless_pass_by_value)]
async extern "Rust" fn chat(
    mut chatbox: String,
    mut console: String,
    handle: Handle,
) -> Result<(String, String)> {
    let _enter = handle.enter();
    let config = crate::config()?;
    let spotify = SPOTIFY.get().context("Spotify is Authenticating...")?;
    let mut lyrics = LYRICS.get().context("Lyrics is Authenticating...")?.clone();

    let current_item = spotify
        .currently_playing_item()
        .send_async()
        .await?
        .context("None")?;

    let public_item = current_item.public_playing_item().context("None")?;
    let PlayingType::Track(track) = public_item.item() else {
        bail!("None")
    };

    if let Ok(color_lyrics) = lyrics.get_color_lyrics(track.id().as_str()).await
        && config.enable_lyrics
    {
        let words = color_lyrics
            .lyrics
            .lines
            .iter()
            .rev()
            .try_find(|line| {
                u64::try_from(public_item.progress().as_millis())
                    .map(|progress| line.start_time_ms < progress)
            })
            .iter()
            .flatten()
            .map(|line| line.words.clone())
            .collect::<Vec<_>>()
            .join(" ");

        if !words.is_empty() && words != "♪" {
            chatbox = words.clone();
            console = words;

            return Ok((chatbox, console));
        }
    }

    replace(&mut chatbox, track);
    replace(&mut console, track);

    let href = track.external_urls().spotify.clone().context("None")?;
    let link = Link::new(&console, &href);
    Ok((chatbox, link.to_string()))
}

#[allow(clippy::literal_string_with_formatting_args)]
fn replace(message: &mut String, track: &FullTrack) {
    let id = &track.id().to_string();
    let song = track.name();
    let artists = track
        .artists()
        .iter()
        .map(CommonArtistInformation::name)
        .collect::<Vec<_>>()
        .join(", ");

    *message = message
        .replace("{id}", id)
        .replace("{song}", song)
        .replace("{artists}", &artists);
}
