#![feature(once_cell_try)]
#![feature(try_find)]

mod chatbox;
mod control;

use std::{net::UdpSocket, sync::OnceLock, time::Duration};

use anyhow::{bail, Result};
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
    prelude::*,
    scope::Scope,
};
use inquire::{Confirm, Text};
use serde::{Deserialize, Serialize};
use spotify_lyrics::{Browser, SpotifyLyrics};
use tiny_http::{Header, Response, Server};
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
    pub client:         String,
    pub format:         String,
    pub redirect_uri:   String,
    pub refresh_token:  String,
    pub secret:         String,
    pub enable_control: bool,
    pub enable_lyrics:  bool,
    pub pkce:           bool,
    pub send_once:      bool,
    pub polling:        u64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            client:         SPOTIFY_CLIENT.into(),
            secret:         SPOTIFY_SECRET.into(),
            redirect_uri:   SPOTIFY_CALLBACK.into(),
            format:         "📻 {song} - {artists}".into(),
            refresh_token:  String::new(),
            enable_control: false,
            enable_lyrics:  true,
            pkce:           false,
            send_once:      true,
            polling:        1,
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

            config.enable_control =
                Confirm::new("Would you like to enable Spotify Controls? (Spotify Premium)")
                    .with_default(config.enable_control)
                    .prompt()?;

            config.enable_lyrics = Confirm::new("Would you like to enable Spotify Lyrics?")
                .with_default(config.enable_lyrics)
                .prompt()?;

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
#[allow(clippy::needless_pass_by_value)]
#[tokio::main(flavor = "current_thread")]
async extern "Rust" fn load(socket: UdpSocket) -> Result<()> {
    let mut config = config()?.clone();
    let mut lyrics = SpotifyLyrics::from_browser(Browser::All)?;
    let spotify = login_to_spotify(&mut config).await?;

    // Disable lyrics if Spotify Lyrics failed to authenticate
    if let Err(error) = lyrics.refresh_authorization().await {
        config.enable_lyrics = false;
        eprintln!("{error}");
    };

    SPOTIFY.set(spotify.clone()).expect("Failed to set SPOTIFY");
    LYRICS.set(lyrics).expect("Failed to set LYRICS");

    if config.enable_control {
        control::start_loop(socket, spotify).await?;
    }

    loop {
        // Keep the threads alive - STATUS_ACCESS_VIOLATION
        tokio::time::sleep(Duration::from_secs(u64::MAX)).await;
    }
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
