use std::net::UdpSocket;

use anyhow::Result;
use rosc::decoder::MTU;
use vrc_osc::config::LoaderConfig;

fn main() -> Result<()> {
    human_panic::setup_panic!();

    let libraries = vrc_osc::get_libraries()?;
    let loader_config = LoaderConfig::load(&libraries)?;
    let loader_socket = UdpSocket::bind(&loader_config.bind_addr)?;
    let plugin_sockets = vrc_osc::load_plugins(libraries, &loader_config)?;

    loop {
        let mut buf = [0u8; MTU];
        let Ok(size) = loader_socket.recv(&mut buf) else {
            continue;
        };

        for local_addr in &plugin_sockets {
            loader_socket.send_to(&buf[..size], local_addr)?;
        }
    }
}
