use abi_stable::{export_root_module, prefix_type::PrefixTypeTrait, sabi_extern_fn};
use common::{Error, OSCMod, OSCMod_Ref};
use error_stack::{IntoReport, Result, ResultExt};
use rosc::{OscMessage, OscPacket, OscType};
use rspotify::{
    model::PlayableItem,
    prelude::{BaseClient, OAuthClient},
    scopes, AuthCodeSpotify, ClientResult, Config as SpotifyConfig, Credentials, OAuth,
};
use serde::{Deserialize, Serialize};
use std::{
    fs::OpenOptions,
    io::{Read, Write},
    net::UdpSocket,
    thread,
    time::Duration,
};
use terminal_link::Link;
use tiny_http::{Header, Response, Server};

const CONFIG_PATH: &str = "spotify.toml";
const SPOTIFY_CLIENT: &str = env!("SPOTIFY_CLIENT");
const SPOTIFY_SECRET: &str = env!("SPOTIFY_SECRET");
const SPOTIFY_CALLBACK: &str = env!("SPOTIFY_CALLBACK");

#[derive(Debug, Serialize, Deserialize)]
struct Config {
    bind_addr: String,
    osc_addr: String,
    client_id: String,
    client_secret: String,
    polling: u64,
}
impl Default for Config {
    fn default() -> Config {
        Config {
            bind_addr: "0.0.0.0:9001".into(),
            osc_addr: "127.0.0.1:9000".into(),
            client_id: SPOTIFY_CLIENT.into(),
            client_secret: SPOTIFY_SECRET.into(),
            polling: 5,
        }
    }
}

#[export_root_module]
fn instantiate_root_module() -> OSCMod_Ref {
    OSCMod { new }.leak_into_prefix()
}

#[sabi_extern_fn]
pub fn new() -> () {
    thread::spawn(|| -> Result<(), Error> {
        let config = load_config()?;

        let osc = UdpSocket::bind(&config.bind_addr)
            .into_report()
            .change_context(Error::IOError)?;

        osc.connect(&config.osc_addr)
            .into_report()
            .change_context(Error::IOError)?;

        let mut spotify = AuthCodeSpotify::with_config(
            Credentials::new(SPOTIFY_CLIENT, SPOTIFY_SECRET),
            OAuth {
                redirect_uri: format!("http://{}", SPOTIFY_CALLBACK),
                scopes: scopes!("user-read-playback-state"),
                ..Default::default()
            },
            SpotifyConfig {
                token_refreshing: true,
                ..Default::default()
            },
        );

        let auth_url = spotify
            .get_authorize_url(false)
            .expect("Failed to get Spotify authorization url");
        prompt_for_token(&mut spotify, &auth_url, SPOTIFY_CALLBACK)
            .expect("Failed to authorize Spotify");

        let mut previous_track = "".to_string();
        loop {
            std::thread::sleep(Duration::from_secs(config.polling));

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
            osc.send(&msg_buf).expect("Failed to send osc message");
        }
    });
}

fn load_config() -> Result<Config, Error> {
    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(CONFIG_PATH)
        .into_report()
        .change_context(Error::IOError)
        .attach_printable(format!("Failed to open {CONFIG_PATH}"))?;
    let mut content = String::new();
    file.read_to_string(&mut content)
        .into_report()
        .change_context(Error::IOError)
        .attach_printable(format!("Failed to read {CONFIG_PATH}"))?;
    match toml::from_str(&content) {
        Ok(config) => Ok(config),
        Err(_) => {
            let config = Config::default();
            let text = toml::to_string(&config)
                .into_report()
                .change_context(Error::TOMLError)?;
            file.write_all(text.as_bytes())
                .into_report()
                .change_context(Error::IOError)?;
            Ok(config)
        }
    }
}

fn prompt_for_token(spotify: &mut AuthCodeSpotify, url: &str, addr: &str) -> ClientResult<()> {
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

fn get_code_from_user(spotify: &AuthCodeSpotify, url: &str, addr: &str) -> ClientResult<String> {
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
