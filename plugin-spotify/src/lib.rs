use std::{net::UdpSocket, sync::Arc, time::Duration};

use anyhow::{bail, Result};
use ferrispot::{
    client::{
        authorization_code::{
            AsyncAuthorizationCodeUserClient,
            AsyncIncompleteAuthorizationCodeUserClient,
        },
        SpotifyClientBuilder,
    },
    prelude::AccessTokenRefreshAsync,
    scope::Scope,
};
use tiny_http::{Header, Response, Server};
use url::Url;

use crate::{chatbox::task_chatbox, config::SpotifyConfig, control::task_control};

mod chatbox;
mod config;
mod control;

#[no_mangle]
#[tokio::main(flavor = "current_thread")]
async fn load(socket: UdpSocket) -> Result<()> {
    let mut config = SpotifyConfig::load()?;
    let user_client = {
        if config.pkce {
            let spotify_client = SpotifyClientBuilder::new(&config.client_id).build_async();
            let user_client = spotify_client
                .authorization_code_client_with_refresh_token_and_pkce(&config.refresh_token)
                .await;

            match user_client {
                Ok(user_client) => user_client,
                Err(_) => {
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
            }
        } else {
            let spotify_client = SpotifyClientBuilder::new(&config.client_id)
                .client_secret(&config.client_secret)
                .build_async()
                .await?;
            let user_client = spotify_client
                .authorization_code_client_with_refresh_token(&config.refresh_token)
                .await;

            match user_client {
                Ok(user_client) => user_client,
                Err(_) => {
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
        }
    };

    println!("Spotify Authenticated");

    let socket = Arc::new(socket);
    if config.enable_control {
        let socket = socket.clone();
        let user_client = user_client.clone();

        tokio::spawn(async move {
            println!("5");
            task_control(socket, user_client)
                .await
                .expect("task_control")
        });
    }

    if config.enable_chatbox {
        tokio::spawn(async move {
            task_chatbox(socket, user_client, config)
                .await
                .expect("task_chatbox")
        });
    }

    loop {
        // Keep the threads alive - STATUS_ACCESS_VIOLATION
        tokio::time::sleep(Duration::from_secs(u64::MAX)).await;
    }
}

async fn finish_authentication_and_save(
    config: &mut SpotifyConfig,
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
            "Error when trying to open an URL in your browser: {:?}. \
             Please navigate here manually: {}",
            why, authorize_url
        ),
    }

    let addr = redirect_uri.replace("http://", "").replace("https://", "");
    let server = Server::http(addr).expect("Failed to bind server");
    let request = server.recv()?;

    let request_url = redirect_uri.to_owned() + request.url();
    let parsed_url = Url::parse(&request_url)?;

    let header = Header::from_bytes(&b"Content-Type"[..], &b"text/html"[..]).unwrap();
    let mut response;
    if parsed_url.query_pairs().count() == 2 {
        response = Response::from_string(
            "<h1>You may close this tab</h1> \
            <script>window.close()</script>",
        );
    } else {
        response = Response::from_string("<h1>An error has occurred</h1>");
    }

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
