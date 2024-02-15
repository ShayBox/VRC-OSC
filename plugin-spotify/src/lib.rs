use std::{net::UdpSocket, sync::Arc, time::Duration};

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
use tiny_http::{Header, Response, Server};
use url::Url;

mod chatbox;
mod control;

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
    pub secret:         String,
    pub redirect_uri:   String,
    pub format:         String,
    pub refresh_token:  String,
    pub enable_chatbox: bool,
    pub enable_control: bool,
    pub pkce:           bool,
    pub send_once:      bool,
    pub send_lyrics:    bool,
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
            enable_chatbox: false,
            enable_control: false,
            pkce:           false,
            send_once:      true,
            send_lyrics:    true,
            polling:        1,
        }
    }
}

#[no_mangle]
#[tokio::main(flavor = "current_thread")]
async extern "Rust" fn load(socket: UdpSocket) -> Result<()> {
    let mut config = if let Ok(config) = Config::load() {
        config
    } else {
        let mut config = Config::default();

        let prompt = "Would you like to enable Spotify Chatbox?";
        config.enable_chatbox = Confirm::new(prompt).with_default(false).prompt()?;

        let prompt = "Would you like to enable Spotify Controls? (Requires Spotify Premium)";
        config.enable_control = Confirm::new(prompt).with_default(false).prompt()?;

        if config.enable_chatbox || config.enable_control {
            println!("The Spotify plugin requires you to create a Spotify Developer Application");
            println!("https://github.com/ShayBox/VRC-OSC/tree/master/plugin-spotify#how-to-setup");
            println!("https://developer.spotify.com/dashboard");

            let prompt = "Spotify Client ID: ";
            config.client = Text::new(prompt).with_default(&config.client).prompt()?;

            let prompt = "Spotify Client Secret: ";
            config.secret = Text::new(prompt).with_default(&config.secret).prompt()?;

            let prompt = "Spotify Redirect URI: ";
            config.redirect_uri = Text::new(prompt)
                .with_default(&config.redirect_uri)
                .prompt()?;
        }

        config.save()?;
        config
    };

    let user_client = {
        if config.pkce {
            let spotify_client = SpotifyClientBuilder::new(&config.client).build_async();
            let user_client = spotify_client
                .authorization_code_client_with_refresh_token_and_pkce(&config.refresh_token)
                .await;

            if let Ok(user_client) = user_client {
                user_client
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

                finish_authentication_and_save(&mut config, incomplete_auth_code_client).await?
            }
        } else {
            let spotify_client = SpotifyClientBuilder::new(&config.client)
                .client_secret(&config.secret)
                .build_async()
                .await?;
            let user_client = spotify_client
                .authorization_code_client_with_refresh_token(&config.refresh_token)
                .await;

            if let Ok(user_client) = user_client {
                user_client
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

                finish_authentication_and_save(&mut config, incomplete_auth_code_client).await?
            }
        }
    };

    println!("Spotify Authenticated");

    let socket = Arc::new(socket);
    if config.enable_control {
        let socket = socket.clone();
        let user_client = user_client.clone();

        tokio::spawn(async move {
            control::task(socket, user_client)
                .await
                .expect("task_control");
        });
    }

    if config.enable_chatbox {
        tokio::spawn(async move {
            chatbox::task(socket, user_client, config)
                .await
                .expect("task_chatbox");
        });
    }

    loop {
        // Keep the threads alive - STATUS_ACCESS_VIOLATION
        tokio::time::sleep(Duration::from_secs(u64::MAX)).await;
    }
}

async fn finish_authentication_and_save(
    config: &mut Config,
    client: AsyncIncompleteAuthorizationCodeUserClient,
) -> Result<AsyncAuthorizationCodeUserClient> {
    let authorize_url = client.get_authorize_url();
    let redirect_uri = &config.redirect_uri;

    let (code, state) = get_user_authorization(&authorize_url, redirect_uri)?;
    let user_client = client.finalize(code.trim(), state.trim()).await?;

    user_client.refresh_access_token().await?;

    config.refresh_token = user_client.get_refresh_token();
    config.save()?;

    Ok(user_client)
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
