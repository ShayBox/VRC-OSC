#![feature(once_cell_try)]
#![feature(try_find)]

use std::{collections::HashMap, net::UdpSocket, sync::OnceLock};

use anyhow::{bail, Context, Result};
use async_ffi::async_ffi;
use derive_config::DeriveTomlConfig;
#[cfg(debug_assertions)]
use dotenvy_macro::dotenv;
use ferrispot::{
    client::{
        authorization_code::{
            AsyncAuthorizationCodeUserClient,
            AsyncIncompleteAuthorizationCodeUserClient,
        },
        SpotifyClientBuilder,
    },
    model::{
        playback::{PlayingType, RepeatState},
        track::FullTrack,
    },
    prelude::*,
    scope::Scope,
};
use inquire::Text;
use rosc::{decoder::MTU, OscMessage, OscPacket, OscType};
use serde::{Deserialize, Serialize};
use spotify_lyrics::{Browser, SpotifyLyrics};
use terminal_link::Link;
use tiny_http::{Header, Response, Server};
use tokio::runtime::Handle;
use url::Url;

#[cfg(debug_assertions)]
const SPOTIFY_CLIENT: &str = dotenv!("SPOTIFY_CLIENT");
#[cfg(debug_assertions)]
const SPOTIFY_SECRET: &str = dotenv!("SPOTIFY_SECRET");
#[cfg(debug_assertions)]
const SPOTIFY_CALLBACK: &str = dotenv!("SPOTIFY_CALLBACK");

#[cfg(not(debug_assertions))]
const SPOTIFY_CLIENT: &str = env!("SPOTIFY_CLIENT");
#[cfg(not(debug_assertions))]
const SPOTIFY_SECRET: &str = env!("SPOTIFY_SECRET");
#[cfg(not(debug_assertions))]
const SPOTIFY_CALLBACK: &str = env!("SPOTIFY_CALLBACK");

#[allow(clippy::struct_excessive_bools)]
#[derive(Clone, Debug, DeriveTomlConfig, Deserialize, Serialize)]
pub struct Config {
    pub client:        String,
    pub secret:        String,
    pub redirect_uri:  String,
    pub format:        String,
    pub refresh_token: String,
    pub pkce:          bool,
    pub send_once:     bool,
    pub send_lyrics:   bool,
    pub polling:       u64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            client:        SPOTIFY_CLIENT.into(),
            secret:        SPOTIFY_SECRET.into(),
            redirect_uri:  SPOTIFY_CALLBACK.into(),
            format:        "📻 {song} - {artists}".into(),
            refresh_token: String::new(),
            pkce:          false,
            send_once:     true,
            send_lyrics:   true,
            polling:       1,
        }
    }
}

static SPOTIFY: OnceLock<AsyncAuthorizationCodeUserClient> = OnceLock::new();
static LYRICS: OnceLock<SpotifyLyrics> = OnceLock::new();
static CONFIG: OnceLock<Config> = OnceLock::new();

fn config() -> Result<&'static Config> {
    CONFIG.get_or_try_init(|| {
        Config::load().or_else(|_| {
            println!("The Spotify plugin requires you to create a Spotify Developer Application");
            println!("https://github.com/ShayBox/VRC-OSC/tree/master/plugin-spotify#how-to-setup");
            println!("https://developer.spotify.com/dashboard");

            let mut config = Config::default();

            config.client = Text::new("Spotify Client ID: ")
                .with_default(&config.client)
                .prompt()?;

            config.secret = Text::new("Spotify Client Secret: ")
                .with_default(&config.secret)
                .prompt()?;

            config.redirect_uri = Text::new("Spotify Redirect URI: ")
                .with_default(&config.redirect_uri)
                .prompt()?;

            config.save()?;

            Ok(config)
        })
    })
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
    let spotify = SPOTIFY.get().context("Authenticating...")?;
    let mut lyrics = LYRICS.get().context("Authenticating...")?.clone();

    let current_item = spotify
        .currently_playing_item()
        .send_async()
        .await?
        .context("None")?;

    let public_item = current_item.public_playing_item().context("None")?;
    let PlayingType::Track(track) = public_item.item() else {
        bail!("None")
    };

    if config.send_lyrics {
        if let Ok(color_lyrics) = lyrics.get_color_lyrics(track.id().as_str()).await {
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

            if words != "♪" {
                chatbox = words.clone();
                console = words;

                return Ok((chatbox, console));
            }
        };
    }

    replace(&mut chatbox, track);
    replace(&mut console, track);

    let href = track.external_urls().spotify.clone().context("None")?;
    let link = Link::new(&console, &href);
    Ok((chatbox, link.to_string()))
}

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

#[no_mangle]
#[allow(clippy::needless_pass_by_value)]
#[tokio::main(flavor = "current_thread")]
async extern "Rust" fn load(socket: UdpSocket) -> Result<()> {
    let mut config = config()?.clone();
    let mut lyrics = SpotifyLyrics::from_browser(Browser::All)?;
    let spotify = login_to_spotify(&mut config).await?;

    // Disable lyrics if Spotify Lyrics failed to authenticate
    if let Err(error) = lyrics.refresh_authorization().await {
        config.send_lyrics = false;
        eprintln!("{error}");
    };

    SPOTIFY.set(spotify.clone()).expect("Failed to set SPOTIFY");
    LYRICS.set(lyrics).expect("Failed to set LYRICS");

    let mut muted_volume = None;
    let mut buf = [0u8; MTU];
    loop {
        let size = socket.recv(&mut buf)?;
        let (_buf, packet) = rosc::decoder::decode_udp(&buf[..size])?;
        let OscPacket::Message(packet) = packet else {
            continue; // I don't think VRChat uses bundles
        };

        let addr = packet.addr.replace("/avatar/parameters/VRCOSC/Media/", "");
        let Some(arg) = packet.args.first() else {
            continue; // No first argument was supplied
        };

        let Some(playback_state) = spotify.playback_state().send_async().await? else {
            continue; // No media is currently playing
        };

        if muted_volume.is_none() {
            muted_volume = Some(playback_state.device().volume_percent());
        }

        match addr.as_ref() {
            "Play" => {
                let OscType::Bool(play) = arg.to_owned() else {
                    continue;
                };

                if play {
                    spotify.resume()
                } else {
                    spotify.pause()
                }
            }
            "Next" => spotify.next(),
            "Prev" | "Previous" => spotify.previous(),
            "Shuffle" => {
                let OscType::Bool(shuffle) = arg.to_owned() else {
                    continue;
                };

                spotify.shuffle(shuffle)
            }
            // Seeking is not required because position is not used multiple times
            // "Seeking" => continue,
            "Muted" => {
                let OscType::Bool(mute) = arg.to_owned() else {
                    continue;
                };

                let volume = if mute {
                    muted_volume = Some(playback_state.device().volume_percent());
                    0
                } else {
                    muted_volume.expect("Failed to get previous volume")
                };

                spotify.volume(volume)
            }
            "Repeat" => {
                let OscType::Int(repeat) = arg.to_owned() else {
                    continue;
                };

                let repeat_state = match repeat {
                    0 => RepeatState::Off,
                    1 => RepeatState::Track,
                    2 => RepeatState::Context,
                    _ => continue,
                };

                spotify.repeat_state(repeat_state)
            }
            "Volume" => {
                let OscType::Float(volume) = arg.to_owned() else {
                    continue;
                };

                #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
                spotify.volume((volume * 100.0) as u8)
            }
            "Position" => {
                let OscType::Float(position) = arg.to_owned() else {
                    continue;
                };

                let min = 0;
                let max = playback_state.currently_playing_item().timestamp();

                #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
                spotify.seek((min + (max - min) * (position * 100.0) as u64) / 100)
            }
            _ => {
                let _ = try_sync_media_state(&socket, &spotify).await;
                continue;
            }
        }
        .send_async()
        .await?;
    }
}

/// Try to synchronize the media session state to the `VRChat` menu parameters
async fn try_sync_media_state(
    socket: &UdpSocket,
    spotify: &AsyncAuthorizationCodeUserClient,
) -> Result<()> {
    let Some(playback_state) = spotify.playback_state().send_async().await? else {
        return Ok(()); // No media is currently playing
    };

    let mut parameters = HashMap::new();

    let play = playback_state.device().is_active();
    parameters.insert("Play", OscType::Bool(play));

    let shuffle = playback_state.shuffle_state();
    parameters.insert("Shuffle", OscType::Bool(shuffle));

    let repeat = match playback_state.repeat_state() {
        RepeatState::Off => 0,
        RepeatState::Track => 1,
        RepeatState::Context => 2,
    };
    parameters.insert("Repeat", OscType::Int(repeat));

    for (param, arg) in parameters {
        let packet = OscPacket::Message(OscMessage {
            addr: format!("/avatar/parameters/VRCOSC/Media/{param}"),
            args: vec![arg],
        });

        let msg_buf = rosc::encoder::encode(&packet)?;
        socket.send(&msg_buf)?;
    }

    Ok(())
}

async fn login_to_spotify(config: &mut Config) -> Result<AsyncAuthorizationCodeUserClient> {
    Ok(if config.pkce {
        let spotify_client = SpotifyClientBuilder::new(&config.client).build_async();

        if let Ok(spotify) = spotify_client
            .authorization_code_client_with_refresh_token_and_pkce(&config.refresh_token)
            .await
        {
            spotify
        } else {
            let incomplete_auth_code_client = spotify_client
                .authorization_code_client_with_pkce(&config.redirect_uri)
                .show_dialog(config.refresh_token.is_empty())
                .scopes([
                    Scope::UserModifyPlaybackState,
                    Scope::UserReadCurrentlyPlaying,
                    Scope::UserReadPlaybackState,
                ])
                .build();

            finish_authentication_and_save(config, incomplete_auth_code_client).await?
        }
    } else {
        let spotify_client = SpotifyClientBuilder::new(&config.client)
            .client_secret(&config.secret)
            .build_async()
            .await?;

        if let Ok(spotify) = spotify_client
            .authorization_code_client_with_refresh_token(&config.refresh_token)
            .await
        {
            spotify
        } else {
            let incomplete_auth_code_client = spotify_client
                .authorization_code_client(&config.redirect_uri)
                .show_dialog(config.refresh_token.is_empty())
                .scopes([
                    Scope::UserModifyPlaybackState,
                    Scope::UserReadCurrentlyPlaying,
                    Scope::UserReadPlaybackState,
                ])
                .build();

            finish_authentication_and_save(config, incomplete_auth_code_client).await?
        }
    })
}

async fn finish_authentication_and_save(
    config: &mut Config,
    client: AsyncIncompleteAuthorizationCodeUserClient,
) -> Result<AsyncAuthorizationCodeUserClient> {
    let authorize_url = client.get_authorize_url();
    let redirect_uri = &config.redirect_uri;

    let (code, state) = get_user_authorization(&authorize_url, redirect_uri)?;
    let spotify = client.finalize(code.trim(), state.trim()).await?;

    spotify.refresh_access_token().await?;

    config.refresh_token = spotify.get_refresh_token();
    config.save()?;

    Ok(spotify)
}

fn get_user_authorization(authorize_url: &str, redirect_uri: &str) -> Result<(String, String)> {
    match webbrowser::open(authorize_url) {
        Ok(ok) => ok,
        Err(why) => eprintln!(
            "Error when trying to open an URL in your browser: {why:?}. \
             Please navigate here manually: {authorize_url}",
        ),
    }

    let addr = redirect_uri.replace("http://", "").replace("https://", "");
    let server = Server::http(addr).expect("Failed to bind server");
    let request = server.recv()?;

    let request_url = redirect_uri.to_owned() + request.url();
    let parsed_url = Url::parse(&request_url)?;

    let header = Header::from_bytes(&b"Content-Type"[..], &b"text/html"[..]).unwrap();
    let mut response = if parsed_url.query_pairs().count() == 2 {
        Response::from_string(
            "<h1>You may close this tab</h1> \
            <script>window.close()</script>",
        )
    } else {
        Response::from_string("<h1>An error has occurred</h1>")
    };

    response.add_header(header);
    request.respond(response).expect("Failed to send response");

    let Some(code) = parsed_url.query_pairs().next() else {
        bail!("None")
    };
    let Some(state) = parsed_url.query_pairs().nth(1) else {
        bail!("None")
    };

    Ok((code.1.into(), state.1.into()))
}
