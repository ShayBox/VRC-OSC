use std::{net::UdpSocket, thread::Builder, time::Duration};

use abi_stable::{
    export_root_module,
    prefix_type::PrefixTypeTrait,
    sabi_extern_fn,
    sabi_trait::TD_Opaque,
};
use anyhow::{bail, Result};
use common::{config::VrcConfig, CommonState_TO, OSCMod, OSCMod_Ref, OscState, StateBox};
use ferrispot::{
    client::{
        authorization_code::{
            SyncAuthorizationCodeUserClient,
            SyncIncompleteAuthorizationCodeUserClient,
        },
        SpotifyClientBuilder,
    },
    model::playback::{PlayingType, RepeatState},
    prelude::{
        AccessTokenRefreshSync,
        CommonArtistInformation,
        CommonTrackInformation,
        ScopedClient,
        SyncRequestBuilder,
    },
    scope::Scope,
};
use rosc::{OscMessage, OscPacket, OscType};
use terminal_link::Link;
use tiny_http::{Header, Response, Server};
use url::Url;

#[export_root_module]
fn instantiate_root_module() -> OSCMod_Ref {
    OSCMod { new }.leak_into_prefix()
}

#[sabi_extern_fn]
pub fn new() -> StateBox {
    let mut config = VrcConfig::load().expect("Failed to load config");
    let osc = UdpSocket::bind("127.0.0.1:0").expect("Failed to bind socket");
    let local_addr = osc.local_addr().expect("Failed to parse local_addr");

    let state = OscState {
        bind_addr: local_addr.to_string().into(),
        send_messages: config.spotify.enable_control,
    };

    if !config.spotify.enable_chatbox && !config.spotify.enable_control {
        println!("Spotify is disabled");
        return CommonState_TO::from_value(state, TD_Opaque);
    }

    let user_client = if config.spotify.pkce {
        let spotify_client = SpotifyClientBuilder::new(&config.spotify.client_id).build_sync();
        let result = spotify_client
            .authorization_code_client_with_refresh_token_and_pkce(&config.spotify.refresh_token);
        match result {
            Ok(user_client) => user_client,
            Err(_) => {
                let incomplete_auth_code_client = spotify_client
                    .authorization_code_client_with_pkce(&config.spotify.redirect_uri)
                    .show_dialog(config.spotify.refresh_token.is_empty())
                    .scopes([
                        Scope::UserModifyPlaybackState,
                        Scope::UserReadCurrentlyPlaying,
                        Scope::UserReadPlaybackState,
                    ])
                    .build();

                finish_authentication_and_save(&mut config, incomplete_auth_code_client).unwrap()
            }
        }
    } else {
        let spotify_client = SpotifyClientBuilder::new(&config.spotify.client_id)
            .client_secret(&config.spotify.client_secret)
            .build_sync()
            .unwrap();
        let result = spotify_client
            .authorization_code_client_with_refresh_token(&config.spotify.refresh_token);
        match result {
            Ok(user_client) => user_client,
            Err(_) => {
                let incomplete_auth_code_client = spotify_client
                    .authorization_code_client(&config.spotify.redirect_uri)
                    .show_dialog(config.spotify.refresh_token.is_empty())
                    .scopes([
                        Scope::UserModifyPlaybackState,
                        Scope::UserReadCurrentlyPlaying,
                        Scope::UserReadPlaybackState,
                    ])
                    .build();

                finish_authentication_and_save(&mut config, incomplete_auth_code_client).unwrap()
            }
        }
    };

    println!("Spotify Authorized");

    if config.spotify.enable_chatbox {
        println!("Spotify Chatbox is enabled");

        let mut config = config.clone();
        let osc = UdpSocket::bind("127.0.0.1:0").expect("Failed to bind socket");
        let user_client = user_client.clone();

        Builder::new()
            .name("Spotify Chatbox".to_string())
            .spawn(move || thread_chatbox(&mut config, &osc, &user_client).expect("thread_chatbox"))
            .expect("Spotify Chatbox failed to spawn");
    } else {
        println!("Spotify Plugin is disabled");
    }

    if config.spotify.enable_control {
        println!("Spotify Control is enabled");
        Builder::new()
            .name("Spotify Control".to_string())
            .spawn(move || thread_control(&mut config, &osc, &user_client).expect("thread_control"))
            .expect("Spotify Control failed to spawn");
    } else {
        println!("Spotify Control is disabled");
    }

    CommonState_TO::from_value(state, TD_Opaque)
}

fn thread_chatbox(
    config: &mut VrcConfig,
    osc: &UdpSocket,
    spotify: &SyncAuthorizationCodeUserClient,
) -> Result<()> {
    let mut previous_track = "".to_string();
    loop {
        std::thread::sleep(Duration::from_secs(config.spotify.polling));

        let Ok(track) = spotify.currently_playing_item().send_sync() else {
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

        let packet = OscPacket::Message(OscMessage {
            addr: "/chatbox/input".into(),
            args: vec![OscType::String(text.into()), OscType::Bool(true)],
        });

        let msg_buf = rosc::encoder::encode(&packet)?;
        osc.send_to(&msg_buf, &config.osc.send_addr)?;
    }
}

fn thread_control(
    _config: &mut VrcConfig,
    osc: &UdpSocket,
    spotify: &SyncAuthorizationCodeUserClient,
) -> Result<()> {
    let mut buf = [0u8; rosc::decoder::MTU];

    let mut previous_volume = None;
    loop {
        let (size, _addr) = osc.recv_from(&mut buf).unwrap();
        let (_buf, packet) = rosc::decoder::decode_udp(&buf[..size]).unwrap();
        let OscPacket::Message(packet) = packet else {
            continue; // I don't think VRChat uses bundles
        };

        let addr = packet.addr.replace("/avatar/parameters/VRCOSC/Media/", "");
        let Some(arg) = packet.args.first() else {
            continue;
        };

        let Some(playback_state) = spotify.playback_state().send_sync()? else {
            continue;
        };

        if previous_volume.is_none() {
            previous_volume = Some(playback_state.device().volume_percent());
        }

        let request = match addr.as_ref() {
            "Play" => spotify.pause(),
            "Next" => spotify.next(),
            "Previous" => spotify.previous(),
            "Shuffle" => {
                if let OscType::Bool(state) = arg.to_owned() {
                    spotify.shuffle(state)
                } else {
                    continue;
                }
            }
            "Muted" => {
                if let OscType::Bool(mute) = arg.to_owned() {
                    if mute {
                        previous_volume = Some(playback_state.device().volume_percent());
                        spotify.volume(0)
                    } else {
                        spotify.volume(previous_volume.expect("Failed to get previous volume"))
                    }
                } else {
                    continue;
                }
            }
            "Repeat" => {
                if let OscType::Int(state) = arg.to_owned() {
                    let repeat_state = match state {
                        0 => RepeatState::Off,
                        1 => RepeatState::Track,
                        2 => RepeatState::Context,
                        _ => continue,
                    };
                    spotify.repeat_state(repeat_state)
                } else {
                    continue;
                }
            }
            "Volume" => {
                if let OscType::Float(volume) = arg.to_owned() {
                    spotify.volume((volume * 100.0) as u8)
                } else {
                    continue;
                }
            }
            _ => continue,
        };
        request.send_sync()?;
    }
}

fn finish_authentication_and_save(
    config: &mut VrcConfig,
    client: SyncIncompleteAuthorizationCodeUserClient,
) -> Result<SyncAuthorizationCodeUserClient> {
    let authorize_url = client.get_authorize_url();
    let redirect_uri = &config.spotify.redirect_uri;

    let (code, state) = get_user_authorization(&authorize_url, redirect_uri)?;
    let user_client = client.finalize(code.trim(), state.trim())?;

    user_client.refresh_access_token()?;

    config.spotify.refresh_token = user_client.get_refresh_token();
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
    let request = match server.recv() {
        Ok(rq) => rq,
        Err(e) => panic!("Failed to get request: {e}"),
    };

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
