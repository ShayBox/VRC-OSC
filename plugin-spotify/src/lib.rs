use abi_stable::{
    export_root_module, prefix_type::PrefixTypeTrait, sabi_extern_fn, sabi_trait::TD_Opaque,
    std_types::RSliceMut,
};
use common::{config::VrcConfig, error::VrcError, OSCMod, OSCMod_Ref, State, StateBox, State_TO};
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

#[derive(Clone, Debug)]
struct SpotifyState {
    enable: bool,
}
impl State for SpotifyState {
    fn is_enabled(&self) -> bool {
        self.enable
    }
}

#[export_root_module]
fn instantiate_root_module() -> OSCMod_Ref {
    OSCMod { new, message }.leak_into_prefix()
}

#[sabi_extern_fn]
pub fn new() -> StateBox {
    let mut config = VrcConfig::load().unwrap();
    let state = SpotifyState {
        enable: config.spotify.enable,
    };

    if config.spotify.enable {
        thread::spawn(move || -> Result<(), VrcError> {
            let osc = UdpSocket::bind("127.0.0.1:0")
                .into_report()
                .change_context(VrcError::Osc)?;

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
                            .scopes([Scope::UserReadPlaybackState])
                            .build();

                        prompt_user_for_authorization(&mut config, incomplete_auth_code_client)?
                    }
                }
            } else {
                let spotify_client = SpotifyClientBuilder::new(&config.spotify.client_id)
                    .client_secret(&config.spotify.client_secret)
                    .build_sync()
                    .into_report()
                    .change_context(VrcError::Spotify)
                    .attach_printable("Failed to build Spotify client")?;

                match spotify_client
                    .authorization_code_client_with_refresh_token(&config.spotify.refresh_token)
                {
                    Ok(user_client) => user_client,
                    Err(_) => {
                        let incomplete_auth_code_client = spotify_client
                            .authorization_code_client(&config.spotify.redirect_uri)
                            .scopes([Scope::UserReadPlaybackState])
                            .build();

                        prompt_user_for_authorization(&mut config, incomplete_auth_code_client)?
                    }
                }
            };

            let mut previous_track = "".to_string();
            loop {
                std::thread::sleep(Duration::from_secs(config.spotify.polling));

                let Some(track) = user_client
                    .currently_playing_track()
                    .into_report()
                    .change_context(VrcError::Spotify)?
                else {
                    continue;
                };

                let Some(item) = track.public_playing_item() else {
                    continue;
                };

                let PlayingType::Track(full_track) = item.item() else {
                    continue;
                };

                let text = format!(
                    "Now Playing: {} by {}",
                    full_track.name(),
                    full_track
                        .artists()
                        .iter()
                        .map(|a| a.name())
                        .collect::<Vec<_>>()
                        .join(", "),
                );

                if full_track.name() != previous_track {
                    previous_track = full_track.name().into();
                    if let Some(href) = &full_track.external_urls().spotify {
                        let link = Link::new(&text, href);
                        println!("{link}");
                    } else {
                        println!("{text}");
                    }
                } else if config.spotify.send_once {
                    continue;
                }

                let message = OscPacket::Message(OscMessage {
                    addr: "/chatbox/input".into(),
                    args: vec![OscType::String(text), OscType::Bool(true)],
                });

                let msg_buf = rosc::encoder::encode(&message)
                    .into_report()
                    .change_context(VrcError::Osc)?;

                osc.send_to(&msg_buf, &config.osc.send_addr)
                    .into_report()
                    .change_context(VrcError::Osc)?;
            }
        });
    }

    State_TO::from_value(state, TD_Opaque)
}

#[sabi_extern_fn]
pub fn message(_state: &StateBox, _size: usize, _buf: RSliceMut<u8>) {}

fn prompt_user_for_authorization(
    config: &mut VrcConfig,
    incomplete_auth_code_client: SyncIncompleteAuthorizationCodeUserClient,
) -> Result<SyncAuthorizationCodeUserClient, VrcError> {
    let authorize_url = incomplete_auth_code_client.get_authorize_url();
    let redirect_uri = &config.spotify.redirect_uri;

    let (code, state) = get_query_from_user(&authorize_url, redirect_uri)?;

    let user_client = incomplete_auth_code_client
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
        response = Response::from_string("<h1>An error has occured</h1>");
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
