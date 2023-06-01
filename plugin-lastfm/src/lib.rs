use std::{net::UdpSocket, time::Duration};

use anyhow::Result;
use rosc::{OscMessage, OscPacket, OscType};
use terminal_link::Link;

use crate::{config::LastFMConfig, model::LastFM};

mod config;
mod model;

#[no_mangle]
fn main(socket: UdpSocket) -> Result<()> {
    let config = LastFMConfig::load()?;
    let mut previous_track = String::new();
    loop {
        let url = format!("https://ws.audioscrobbler.com/2.0/?method=user.getrecenttracks&user={}&api_key={}&format=json&limit=1", config.username, config.api_key);
        let response = reqwest::blocking::get(url)?;
        let lastfm = response.json::<LastFM>()?;
        let tracks = lastfm
            .recent
            .tracks
            .iter()
            .filter(|track| {
                if let Some(attr) = &track.attr {
                    attr.nowplaying
                } else {
                    false
                }
            })
            .collect::<Vec<_>>();

        let Some(track) = tracks.first() else {
            continue;
        };

        let text = &config
            .format
            .replace("{song}", &track.name)
            .replace("{artists}", &track.artist.text);

        if track.name != previous_track {
            previous_track = track.name.to_owned();
            let link = Link::new(text, &track.url);
            println!("{link}");
        } else if config.send_once {
            continue;
        }

        let packet = OscPacket::Message(OscMessage {
            addr: "/chatbox/input".into(),
            args: vec![OscType::String(text.into()), OscType::Bool(true)],
        });

        let msg_buf = rosc::encoder::encode(&packet)?;
        socket.send(&msg_buf)?;

        std::thread::sleep(Duration::from_secs(config.polling));
    }
}
