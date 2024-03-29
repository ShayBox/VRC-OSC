use std::net::UdpSocket;

use anyhow::Result;
use derive_config::DeriveTomlConfig;
use inquire::Confirm;
use loader::{Config, CARGO_PKG_HOMEPAGE};
use rosc::decoder::MTU;
use terminal_link::Link;

#[tokio::main]
async fn main() -> Result<()> {
    human_panic::setup_panic!();

    if loader::check_for_updates()? {
        let link = Link::new("An update is available", CARGO_PKG_HOMEPAGE);
        println!("{link}");
    }

    let config = if let Ok(config) = Config::load() {
        config
    } else {
        let mut config = Config::default();
        let mut plugins = loader::get_plugin_names()?;
        plugins.sort();

        for plugin in plugins {
            let prompt = format!("Would you like to enable the {plugin} plugin");
            if Confirm::new(&prompt).with_default(false).prompt()? {
                config.enabled.push(plugin.clone());
            }
        }

        if config.enabled.is_empty() {
            println!("You must enable at least one plugin");
            std::process::exit(1);
        }

        config.save()?;
        config
    };

    let loader_socket = UdpSocket::bind(&config.bind_addr)?;
    let plugin_names = loader::get_plugin_names()?;
    let plugin_addrs = loader::load_plugins(plugin_names, &config)?;

    loop {
        let mut buf = [0u8; MTU];
        let Ok((size, recv_addr)) = loader_socket.recv_from(&mut buf) else {
            continue;
        };

        // Plugins -> VRChat
        if plugin_addrs.contains(&recv_addr) {
            loader_socket.send_to(&buf[..size], &config.send_addr)?;
            continue;
        }

        // VRChat -> Plugins
        for plugin_addr in &plugin_addrs {
            loader_socket.send_to(&buf[..size], plugin_addr)?;
        }
    }
}
