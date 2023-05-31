use std::net::UdpSocket;

use anyhow::Result;
use common::config::VrcConfig;

fn main() -> Result<()> {
    human_panic::setup_panic!();

    let config = VrcConfig::load()?;
    if config.lastfm.enable && config.spotify.enable_chatbox {
        println!("You cannot enable LastFM and Spotify Chatbox at the same time.");
        return Ok(());
    }

    let osc = UdpSocket::bind(&config.osc.bind_addr)?;
    let plugins = vrc_osc::load_plugins()?;

    loop {
        loop {
            let mut buf = [0u8; rosc::decoder::MTU];
            let Ok(size) = osc.recv(&mut buf) else {
                continue;
            };

            for (_plugin, state) in plugins.values() {
                let bind_addr = state.bind_addr().to_string();
                osc.send_to(&buf[..size], bind_addr)?;
            }
        }
    }
}
