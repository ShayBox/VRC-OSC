use std::net::UdpSocket;

use anyhow::Result;
use rosc::decoder::MTU;
use vrc_osc::config::LoaderConfig;

#[tokio::main]
async fn main() -> Result<()> {
    human_panic::setup_panic!();

    let libraries = vrc_osc::get_libraries()?;
    let loader_config = LoaderConfig::load(&libraries)?;
    let loader_socket = UdpSocket::bind(&loader_config.bind_addr)?;
    let plugin_addrs = vrc_osc::load_plugins(libraries, &loader_config)?;

    loop {
        let mut buf = [0u8; MTU];
        let Ok((size, recv_addr)) = loader_socket.recv_from(&mut buf) else {
            continue;
        };

        // Plugins -> VRChat
        if plugin_addrs.contains(&recv_addr) {
            loader_socket.send_to(&buf[..size], &loader_config.send_addr)?;
            continue;
        }

        // VRChat -> Plugins
        for plugin_addr in &plugin_addrs {
            loader_socket.send_to(&buf[..size], plugin_addr)?;
        }
    }
}
