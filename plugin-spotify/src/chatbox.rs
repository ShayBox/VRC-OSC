use std::{net::UdpSocket, sync::Arc, time::Duration};

use anyhow::Result;
use ferrispot::{
    client::authorization_code::AsyncAuthorizationCodeUserClient,
    model::playback::PlayingType,
    prelude::{CommonArtistInformation, *},
};
use rosc::{OscMessage, OscPacket, OscType};
use spotify_lyrics::{Browser, SpotifyLyrics};
use terminal_link::Link;

use crate::Config;

pub async fn task(
    socket: Arc<UdpSocket>,
    spotify: AsyncAuthorizationCodeUserClient,
    mut config: Config,
) -> Result<()> {
    let mut previous_lyrics = None;
    let mut previous_track = String::new();
    let mut previous_words = String::new();
    let mut spotify_lyrics = SpotifyLyrics::from_browser(Browser::All)?;

    // Disable lyrics if Spotify Lyrics failed to authenticate
    if let Err(error) = spotify_lyrics.refresh_authorization().await {
        config.send_lyrics = false;
        eprintln!("{error}");
    };

    loop {
        std::thread::sleep(Duration::from_secs(config.polling));

        let Ok(track) = spotify.currently_playing_item().send_async().await else {
            continue;
        };

        let Some(track) = track else {
            continue;
        };

        let Some(item) = track.public_playing_item() else {
            continue;
        };

        let PlayingType::Track(full_track) = item.item() else {
            continue;
        };

        let id = &full_track.id().to_string();
        let song = full_track.name();
        let artists = full_track
            .artists()
            .iter()
            .map(CommonArtistInformation::name)
            .collect::<Vec<_>>()
            .join(", ");

        let text = &config
            .format
            .replace("{id}", id)
            .replace("{song}", song)
            .replace("{artists}", &artists);

        if full_track.name() != previous_track {
            previous_track = full_track.name().into();

            if let Some(href) = &full_track.external_urls().spotify {
                let link = Link::new(text, href);
                println!("{link}");
            } else {
                println!("{text}");
            }

            if config.send_lyrics {
                previous_lyrics = spotify_lyrics.get_color_lyrics(id).await.ok();
            }
        } else if config.send_lyrics {
            let Some(color_lyrics) = &previous_lyrics else {
                continue;
            };

            #[allow(clippy::cast_possible_truncation)]
            let Some(current_words) = color_lyrics
                .lyrics
                .lines
                .iter()
                .rev()
                .find(|line| line.start_time_ms < item.progress().as_millis() as u64)
                .map(|line| line.words.clone())
            else {
                continue;
            };

            if current_words == previous_words {
                continue;
            }

            previous_words = current_words;

            let message = OscMessage {
                addr: "/chatbox/input".into(),
                args: vec![OscType::String(previous_words.clone()), OscType::Bool(true)],
            };

            let packet = OscPacket::Message(message);
            let msg_buf = rosc::encoder::encode(&packet)?;

            socket.send(&msg_buf)?;

            if previous_words != "♪" {
                println!("{previous_words}");
                continue;
            }
        } else if config.send_once {
            continue;
        }

        let message = OscMessage {
            addr: "/chatbox/input".into(),
            args: vec![OscType::String(text.into()), OscType::Bool(true)],
        };

        let packet = OscPacket::Message(message);
        let msg_buf = rosc::encoder::encode(&packet)?;

        socket.send(&msg_buf)?;
    }
}
