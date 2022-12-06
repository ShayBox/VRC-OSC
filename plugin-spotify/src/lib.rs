use abi_stable::{
    export_root_module, prefix_type::PrefixTypeTrait, sabi_extern_fn, sabi_trait::TD_Opaque,
};
use common::{
    config::VrcConfig, error::VrcError, CommonState_TO, OSCMod, OSCMod_Ref, OscState, StateBox,
};
use error_stack::{bail, IntoReport, Result, ResultExt};
use ferrispot::{
    client::{
        authorization_code::{
            SyncAuthorizationCodeUserClient, SyncIncompleteAuthorizationCodeUserClient,
        },
        SpotifyClientBuilder,
    },
    model::playback::PlayingType,
    prelude::{
        AccessTokenRefreshSync, CommonArtistInformation, CommonTrackInformation, ScopedSyncClient,
    },
    scope::Scope,
};
use rosc::{OscMessage, OscPacket, OscType};
use std::{net::UdpSocket, thread, time::Duration};
use terminal_link::Link;
use tiny_http::{Header, Response, Server};
use url::Url;

#[export_root_module]
fn instantiate_root_module() -> OSCMod_Ref {
    OSCMod { new }.leak_into_prefix()
}

#[sabi_extern_fn]
pub fn new() -> StateBox {
    let mut config = VrcConfig::load().unwrap();

    let osc = UdpSocket::bind("127.0.0.1:0")
        .into_report()
        .change_context(VrcError::Osc)
        .unwrap();

    let local_addr = osc
        .local_addr()
        .into_report()
        .change_context(VrcError::Io)
        .unwrap();

    let state = OscState {
        bind_addr: local_addr.to_string().into(),
        send_messages: false,
    };

    if config.spotify.enable {
        println!("Spotify Plugin is enabled");
        thread::Builder::new()
            .name("Spotify Plugin".to_string())
            .spawn(move || {
                let user_client = if config.spotify.pkce {
                    let spotify_client =
                        SpotifyClientBuilder::new(&config.spotify.client_id).build_sync();

                    match spotify_client.authorization_code_client_with_refresh_token_and_pkce(
                        &config.spotify.refresh_token,
                    ) {
                        Ok(user_client) => user_client,
                        Err(_) => {
                            let incomplete_auth_code_client = spotify_client
                                .authorization_code_client_with_pkce(&config.spotify.redirect_uri)
                                .show_dialog(config.spotify.refresh_token.is_empty())
                                .scopes([Scope::UserReadCurrentlyPlaying])
                                .build();

                            prompt_user_for_authorization(&mut config, incomplete_auth_code_client)
                                .unwrap()
                        }
                    }
                } else {
                    let spotify_client = SpotifyClientBuilder::new(&config.spotify.client_id)
                        .client_secret(&config.spotify.client_secret)
                        .build_sync()
                        .into_report()
                        .change_context(VrcError::Spotify)
                        .attach_printable("Failed to build Spotify client")
                        .unwrap();

                    match spotify_client
                        .authorization_code_client_with_refresh_token(&config.spotify.refresh_token)
                    {
                        Ok(user_client) => user_client,
                        Err(_) => {
                            let incomplete_auth_code_client = spotify_client
                                .authorization_code_client(&config.spotify.redirect_uri)
                                .show_dialog(config.spotify.refresh_token.is_empty())
                                .scopes([Scope::UserReadCurrentlyPlaying])
                                .build();

                            prompt_user_for_authorization(&mut config, incomplete_auth_code_client)
                                .unwrap()
                        }
                    }
                };

                println!("Loaded Spotify");

                let mut previous_track = "".to_string();
                loop {
                    std::thread::sleep(Duration::from_secs(config.spotify.polling));

                    let Some(track) = user_client
                        .currently_playing_item()
                        .into_report()
                        .change_context(VrcError::Spotify)
                        .unwrap()
                    else {
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
                        .spotify
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
                    } else if config.spotify.send_once {
                        continue;
                    }

                    let message = OscPacket::Message(OscMessage {
                        addr: "/chatbox/input".into(),
                        args: vec![OscType::String(text.into()), OscType::Bool(true)],
                    });

                    let msg_buf = rosc::encoder::encode(&message)
                        .into_report()
                        .change_context(VrcError::Osc)
                        .unwrap();

                    osc.send_to(&msg_buf, &config.osc.send_addr)
                        .into_report()
                        .change_context(VrcError::Osc)
                        .unwrap();
                }
            })
            .expect("Spotify Plugin failed");
    } else {
        println!("Spotify Plugin is disabled");
    }

    CommonState_TO::from_value(state, TD_Opaque)
}

fn prompt_user_for_authorization(
    config: &mut VrcConfig,
    client: SyncIncompleteAuthorizationCodeUserClient,
) -> Result<SyncAuthorizationCodeUserClient, VrcError> {
    let authorize_url = client.get_authorize_url();
    let redirect_uri = &config.spotify.redirect_uri;
    let (code, state) = get_query_from_user(&authorize_url, redirect_uri)?;
    let user_client = client
        .finalize(code.trim(), state.trim())
        .into_report()
        .change_context(VrcError::Spotify)?;

    user_client
        .refresh_access_token()
        .into_report()
        .change_context(VrcError::Spotify)?;

    config.spotify.refresh_token = user_client.get_refresh_token();
    config.save()?;

    Ok(user_client)
}

fn get_query_from_user(url: &str, uri: &str) -> Result<(String, String), VrcError> {
    match webbrowser::open(url) {
        Ok(ok) => ok,
        Err(why) => eprintln!(
            "Error when trying to open an URL in your browser: {:?}. \
             Please navigate here manually: {}",
            why, url
        ),
    }

    let addr = uri.replace("http://", "").replace("https://", "");
    let server = Server::http(addr).expect("Failed to bind server");
    let request = match server.recv() {
        Ok(rq) => rq,
        Err(e) => panic!("Failed to get request: {e}"),
    };

    let request_url = uri.to_owned() + request.url();
    let parsed_url = Url::parse(&request_url)
        .into_report()
        .change_context(VrcError::Url)?;

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
        bail!(VrcError::None)
    };
    let Some(state) = parsed_url.query_pairs().nth(1) else {
        bail!(VrcError::None)
    };

    Ok((code.1.into(), state.1.into()))
}
