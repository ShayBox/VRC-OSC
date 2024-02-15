use std::{net::UdpSocket, time::Duration};

use anyhow::Result;
use derive_config::DeriveTomlConfig;
#[cfg(debug_assertions)]
use dotenvy_macro::dotenv;
use inquire::Text;
use rosc::{OscMessage, OscPacket, OscType};
use serde::{Deserialize, Serialize};
use terminal_link::Link;

use crate::model::LastFM;

mod model;

#[cfg(debug_assertions)]
const LASTFM_API_KEY: &str = dotenv!("LASTFM_API_KEY");
#[cfg(debug_assertions)]
const LASTFM_USERNAME: &str = dotenv!("LASTFM_USERNAME");

#[cfg(not(debug_assertions))]
const LASTFM_API_KEY: &str = env!("LASTFM_API_KEY");
#[cfg(not(debug_assertions))]
const LASTFM_USERNAME: &str = env!("LASTFM_USERNAME");

#[derive(Clone, Debug, DeriveTomlConfig, Deserialize, Serialize)]
pub struct Config {
    pub api_key:   String,
    pub username:  String,
    pub format:    String,
    pub send_once: bool,
    pub polling:   u64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            api_key:   LASTFM_API_KEY.into(),
            username:  LASTFM_USERNAME.into(),
            format:    "📻 {song} - {artists}".into(),
            send_once: false,
            polling:   10,
        }
    }
}

#[no_mangle]
#[allow(clippy::needless_pass_by_value)]
#[tokio::main(flavor = "current_thread")]
async extern "Rust" fn load(socket: UdpSocket) -> Result<()> {
    let config = if let Ok(config) = Config::load() {
        config
    } else {
        println!("The LastFM plugin requires you to setup a scrobbler app or service");
        println!("https://www.last.fm/about/trackmymusic");

        Config {
            username: Text::new("LastFM Username: ").prompt()?,
            ..Default::default()
        }
    };

    let mut previous_track = String::new();
    loop {
        std::thread::sleep(Duration::from_secs(config.polling));

        let url = format!("http://ws.audioscrobbler.com/2.0/?method=user.getrecenttracks&user={}&api_key={}&format=json&limit=1", config.username, config.api_key);
        let response = reqwest::get(url).await?;
        let lastfm = response.json::<LastFM>().await?;
        let tracks = lastfm
            .recent
            .tracks
            .iter()
            .filter(|track| track.attr.as_ref().map_or(false, |attr| attr.nowplaying))
            .collect::<Vec<_>>();

        let Some(track) = tracks.first() else {
            continue;
        };

        let text = &config
            .format
            .replace("{song}", &track.name)
            .replace("{artists}", &track.artist.text);

        if track.name != previous_track {
            previous_track = track.name.clone();
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
    }
}
