#![feature(once_cell_try)]

mod model;

use std::{net::UdpSocket, sync::OnceLock};

use anyhow::{Context, Error, Result};
use async_ffi::async_ffi;
use derive_config::DeriveTomlConfig;
#[cfg(debug_assertions)]
use dotenvy_macro::dotenv;
use inquire::Text;
use model::Track;
use serde::{Deserialize, Serialize};
use terminal_link::Link;
use tokio::runtime::Handle;

use crate::model::LastFM;

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
    pub api_key:  String,
    pub username: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            api_key:  LASTFM_API_KEY.into(),
            username: LASTFM_USERNAME.into(),
        }
    }
}

fn config() -> Result<&'static Config> {
    static CONFIG: OnceLock<Config> = OnceLock::new();
    CONFIG.get_or_try_init(|| {
        Config::load().or_else(|_| {
            println!("The LastFM plugin requires you to setup a scrobbler app or service");
            println!("https://www.last.fm/about/trackmymusic");

            Ok::<Config, Error>(Config {
                username: Text::new("LastFM Username: ").prompt()?,
                ..Default::default()
            })
        })
    })
}

#[no_mangle]
#[allow(clippy::needless_pass_by_value)]
#[tokio::main(flavor = "current_thread")]
async extern "Rust" fn load(_: UdpSocket) -> Result<()> {
    config()?.save()?;

    Ok(())
}

#[no_mangle]
#[async_ffi(?Send)]
#[allow(clippy::unnecessary_wraps)]
#[allow(clippy::needless_pass_by_value)]
async extern "Rust" fn chat(
    mut chatbox: String,
    mut console: String,
    handle: Handle,
) -> Result<(String, String)> {
    let _enter = handle.enter();
    let config = config()?;
    let url = format!("http://ws.audioscrobbler.com/2.0/?method=user.getrecenttracks&user={}&api_key={}&format=json&limit=1", config.username, config.api_key);
    let response = ureq::get(&url).call()?;
    let lastfm = response.into_json::<LastFM>()?;
    let tracks = lastfm
        .recent
        .tracks
        .iter()
        .filter(|track| track.attr.as_ref().map_or(false, |attr| attr.nowplaying))
        .collect::<Vec<_>>();

    let track = tracks.first().context("No track found")?;
    replace(&mut chatbox, track);
    replace(&mut console, track);

    let link = Link::new(&console, &track.url);
    Ok((chatbox, link.to_string()))
}

fn replace(message: &mut String, track: &Track) {
    *message = message
        .replace("{song}", &track.name)
        .replace("{artist}", &track.artist.text)
        .replace("{artists}", &track.artist.text);
}
