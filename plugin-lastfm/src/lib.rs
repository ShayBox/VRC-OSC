use std::{net::UdpSocket, thread::Builder, time::Duration};

use abi_stable::{
    export_root_module,
    prefix_type::PrefixTypeTrait,
    sabi_extern_fn,
    sabi_trait::TD_Opaque,
};
use anyhow::Result;
use common::{config::VrcConfig, CommonState_TO, OSCMod, OSCMod_Ref, OscState, StateBox};
use rosc::{OscMessage, OscPacket, OscType};
use terminal_link::Link;

use crate::json::LastFM;

mod json;

#[export_root_module]
fn instantiate_root_module() -> OSCMod_Ref {
    OSCMod { new }.leak_into_prefix()
}

#[sabi_extern_fn]
pub fn new() -> StateBox {
    let config = VrcConfig::load().expect("Failed to load config");
    let osc = UdpSocket::bind("127.0.0.1:0").expect("Failed to bind socket");
    let local_addr = osc.local_addr().expect("Failed to parse local_addr");

    let state = OscState {
        bind_addr: local_addr.to_string().into(),
        send_messages: false,
    };

    if config.lastfm.enable {
        println!("LastFM is enabled");
        Builder::new()
            .name("LastFM Plugin".to_string())
            .spawn(move || thread_lastfm(config, &osc).expect("thread_lastfm"))
            .expect("LastFM Plugin failed");
    } else {
        println!("LastFM is disabled");
    }

    CommonState_TO::from_value(state, TD_Opaque)
}

fn thread_lastfm(config: VrcConfig, osc: &UdpSocket) -> Result<()> {
    let mut previous_track = "".to_string();
    loop {
        std::thread::sleep(Duration::from_secs(config.lastfm.polling));

        let url = format!("http://ws.audioscrobbler.com/2.0/?method=user.getrecenttracks&user={}&api_key={}&format=json&limit=1", config.lastfm.username, config.lastfm.api_key);
        let response = reqwest::blocking::get(url)?;
        let lastfm = response.json::<LastFM>()?;
        let tracks = lastfm
            .recent
            .tracks
            .iter()
            .filter(|track| {
                if let Some(attr) = &track.attr {
                    attr.nowplaying
                } else {
                    false
                }
            })
            .collect::<Vec<_>>();

        let Some(track) = tracks.first() else {
            continue;
        };

        let text = &config
            .lastfm
            .format
            .replace("{song}", &track.name)
            .replace("{artists}", &track.artist.text);

        if track.name != previous_track {
            previous_track = track.name.to_owned();
            let link = Link::new(text, &track.url);
            println!("{link}");
        } else if config.lastfm.send_once {
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
