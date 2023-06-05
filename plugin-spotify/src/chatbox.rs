use std::{net::UdpSocket, sync::Arc, time::Duration};

use anyhow::Result;
use ferrispot::{
    client::authorization_code::AsyncAuthorizationCodeUserClient,
    model::playback::PlayingType,
    prelude::*,
};
use rosc::{OscMessage, OscPacket, OscType};
use terminal_link::Link;

use crate::config::SpotifyConfig;

pub async fn task_chatbox(
    socket: Arc<UdpSocket>,
    spotify: AsyncAuthorizationCodeUserClient,
    config: SpotifyConfig,
) -> Result<()> {
    let mut previous_track = String::new();
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

        let song = full_track.name();
        let artists = full_track
            .artists()
            .iter()
            .map(|a| a.name())
            .collect::<Vec<_>>()
            .join(", ");

        let text = &config
            .format
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
        } else if config.send_once {
            continue;
        }

        let packet = OscPacket::Message(OscMessage {
            addr: "/chatbox/input".into(),
            args: vec![OscType::String(text.into()), OscType::Bool(true)],
        });

        let msg_buf = rosc::encoder::encode(&packet)?;
        socket.send(&msg_buf)?;
    }
}
