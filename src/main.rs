use clap::Parser;
use clap_verbosity_flag::Verbosity;
use rosc::{encoder, OscMessage, OscPacket, OscType};
use rspotify::{
    model::PlayableItem, prelude::*, scopes, AuthCodeSpotify, ClientResult, Config, Credentials,
    OAuth,
};
use std::{net::UdpSocket, time::Duration};
use terminal_link::Link;
use tiny_http::{Header, Response, Server};

#[macro_use]
extern crate log;

#[derive(Debug, Parser)]
struct Args {
    /// Address to connect to VRChat OSC device or computer
    #[arg(short, long, default_value = "127.0.0.1:9000")]
    osc_addr: String,

    /// Polling interval in seconds
    #[arg(short, long, default_value_t = 5)]
    polling: u64,

    #[clap(flatten)]
    verbose: Verbosity,
}

fn main() {
    let args = Args::parse();

    env_logger::Builder::new()
        .filter_level(args.verbose.log_level_filter())
        .init();
    info!("Initialized Logger");

    let osc = UdpSocket::bind("127.0.0.1:0").expect("Failed to bind local OSC UdpSocket");
    osc.connect(&args.osc_addr)
        .expect("Failed to connect to VRChat via OSC");
    info!("Initialized OSC");

    let client = env!("SPOTIFY_CLIENT");
    let secret = env!("SPOTIFY_SECRET");
    let addr = option_env!("SPOTIFY_CALLBACK").unwrap_or("127.0.0.1:2345");
    let credentials = Credentials::new(client, secret);
    let oauth = OAuth {
        redirect_uri: format!("http://{}", &addr),
        scopes: scopes!("user-read-playback-state"),
        ..Default::default()
    };
    let config = Config {
        token_refreshing: true,
        ..Default::default()
    };
    let mut spotify = AuthCodeSpotify::with_config(credentials, oauth, config);
    info!("Initialized Spotify");

    let url = spotify
        .get_authorize_url(false)
        .expect("Failed to get Spotify authorization url");
    prompt_for_token(&mut spotify, &url, &addr).expect("Failed to authorize Spotify");
    info!("Authorized Spotify");

    let mut previous_track = "".to_string();
    loop {
        std::thread::sleep(Duration::from_secs(args.polling));

        let playing = spotify
            .current_user_playing_item()
            .expect("Failed to get users currently playing item");

        let playing = match playing {
            Some(playing) => playing,
            None => continue,
        };

        let item = match playing.item {
            Some(item) => item,
            None => continue,
        };

        let track = match item {
            PlayableItem::Track(track) => track,
            _ => continue,
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

        let msg_buf = encoder::encode(&OscPacket::Message(OscMessage {
            addr: "/chatbox/input".to_string(),
            args: vec![
                OscType::String(text),
                OscType::Bool(true),
            ],
        }))
        .expect("Failed to encode osc message");
        osc.send(&msg_buf).expect("Failed to send osc message");
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
                        log::info!("Successfully refreshed expired token from token cache");
                        *spotify.get_token().lock().unwrap() = Some(refreshed_token)
                    }
                    // If not, prompt the user for it
                    None => {
                        log::info!("Unable to refresh expired token from token cache");
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

    info!("Opening brower with auth URL");
    match webbrowser::open(url) {
        Ok(_) => info!("Opened authorization in your browser."),
        Err(why) => eprintln!(
            "Error when trying to open an URL in your browser: {:?}. \
             Please navigate here manually: {}",
            why, url
        ),
    }

    info!("Please accept Spotify Authorization in your browser");
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
