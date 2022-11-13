use abi_stable::{
    export_root_module, prefix_type::PrefixTypeTrait, sabi_extern_fn, sabi_trait::TD_Opaque,
    std_types::RSliceMut,
};
use common::{config::VrcConfig, error::VRCError, OSCMod, OSCMod_Ref, State, StateBox, State_TO};
use error_stack::{IntoReport, Result, ResultExt};
use rosc::{OscMessage, OscPacket, OscType};
use rspotify::{
    model::PlayableItem,
    prelude::{BaseClient, OAuthClient},
    scopes, AuthCodePkceSpotify, ClientResult, Config as SpotifyConfig, Credentials, OAuth,
};
use std::{net::UdpSocket, thread, time::Duration};
use terminal_link::Link;
use tiny_http::{Header, Response, Server};

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
    let config = VrcConfig::load().unwrap();
    let state = SpotifyState {
        enable: config.spotify.enable,
    };

    if config.spotify.enable {
        thread::spawn(move || -> Result<(), VRCError> {
            let osc = UdpSocket::bind("127.0.0.1:0")
                .into_report()
                .change_context(VRCError::IOError)?;

            let mut spotify = AuthCodePkceSpotify::with_config(
                Credentials::new_pkce(&config.spotify.client_id),
                OAuth {
                    redirect_uri: format!("http://{}", &config.spotify.callback_uri),
                    scopes: scopes!("user-read-playback-state"),
                    ..Default::default()
                },
                SpotifyConfig {
                    token_refreshing: true,
                    ..Default::default()
                },
            );

            let auth_url = spotify
                .get_authorize_url(None)
                .expect("Failed to get Spotify authorization url");
            prompt_for_token(&mut spotify, &auth_url, &config.spotify.callback_uri)
                .expect("Failed to authorize Spotify");

            let mut previous_track = "".to_string();
            loop {
                std::thread::sleep(Duration::from_secs(config.spotify.polling));

                let playing = spotify
                    .current_user_playing_item()
                    .expect("Failed to get users currently playing item");

                let Some(playing) = playing else {
                continue;
            };
                let Some(item) = playing.item else {
                continue;
            };
                let PlayableItem::Track(track) = item else {
                continue;
            };

                if track.name == previous_track {
                    continue;
                } else {
                    previous_track = track.name.to_owned();
                }

                let artists = track
                    .artists
                    .iter()
                    .map(|a| a.name.to_owned())
                    .collect::<Vec<_>>()
                    .join(", ");

                let text = format!("Now Playing: {} by {}", track.name, artists);
                if let Some(href) = track.href {
                    let link = Link::new(&text, &href);
                    println!("{link}");
                } else {
                    println!("{text}");
                }

                let msg_buf = rosc::encoder::encode(&OscPacket::Message(OscMessage {
                    addr: "/chatbox/input".to_string(),
                    args: vec![OscType::String(text), OscType::Bool(true)],
                }))
                .expect("Failed to encode osc message");
                osc.send_to(&msg_buf, &config.osc.send_addr)
                    .expect("Failed to send osc message");
            }
        });
    }

    State_TO::from_value(state, TD_Opaque)
}

#[sabi_extern_fn]
pub fn message(_state: &StateBox, _size: usize, _buf: RSliceMut<u8>) -> () {}

fn prompt_for_token(spotify: &mut AuthCodePkceSpotify, url: &str, addr: &str) -> ClientResult<()> {
    match spotify.read_token_cache(true) {
        Ok(Some(new_token)) => {
            let expired = new_token.is_expired();

            // Load token into client regardless of whether it's expired o
            // not, since it will be refreshed later anyway.
            *spotify.get_token().lock().unwrap() = Some(new_token);

            if expired {
                // Ensure that we actually got a token from the refetch
                match spotify.refetch_token()? {
                    Some(refreshed_token) => {
                        *spotify.get_token().lock().unwrap() = Some(refreshed_token)
                    }
                    // If not, prompt the user for it
                    None => {
                        let code = get_code_from_user(spotify, url, addr)?;
                        spotify.request_token(&code)?;
                    }
                }
            }
        }
        // Otherwise following the usual procedure to get the token.
        _ => {
            let code = get_code_from_user(spotify, url, addr)?;
            spotify.request_token(&code)?;
        }
    }

    spotify.write_token_cache()
}

fn get_code_from_user(
    spotify: &AuthCodePkceSpotify,
    url: &str,
    addr: &str,
) -> ClientResult<String> {
    use rspotify::ClientError;

    match webbrowser::open(url) {
        Ok(ok) => ok,
        Err(why) => eprintln!(
            "Error when trying to open an URL in your browser: {:?}. \
             Please navigate here manually: {}",
            why, url
        ),
    }

    let server = Server::http(&addr).expect("Failed to bind server");
    let request = match server.recv() {
        Ok(rq) => rq,
        Err(e) => panic!("Failed to get request: {e}"),
    };
    let url = format!("http://{addr}{}", request.url());
    let code = spotify
        .parse_response_code(&url)
        .ok_or_else(|| ClientError::Cli("unable to parse the response code".to_string()))?;
    let mut response = Response::from_string(
        " \
            <h1>You may close this tab</h1> \
            <script>window.close()</script> \
        ",
    );
    let header = Header::from_bytes(&b"Content-Type"[..], &b"text/html"[..])
        .expect("Failed to parse header");
    response.add_header(header);
    request.respond(response).expect("Failed to send response");

    Ok(code)
}
