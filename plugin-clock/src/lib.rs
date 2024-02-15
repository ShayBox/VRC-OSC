#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_precision_loss)]

use std::{
    collections::HashMap,
    net::UdpSocket,
    time::{Duration, SystemTime},
};

use anyhow::Result;
use derive_config::DeriveTomlConfig;
use rosc::{OscMessage, OscPacket, OscType};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, DeriveTomlConfig, Deserialize, Serialize)]
pub struct Config {
    pub mode:    bool,
    pub polling: u64,
    pub smooth:  bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            mode:    false,
            polling: 1000,
            smooth:  false,
        }
    }
}

#[no_mangle]
#[allow(clippy::needless_pass_by_value)]
#[tokio::main(flavor = "current_thread")]
async extern "Rust" fn load(socket: UdpSocket) -> Result<()> {
    let config = Config::load()?;

    loop {
        let duration = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)?;
        let seconds = duration.as_secs();

        let mut hours = (seconds / 3600) as f64;
        let mut minutes = ((seconds % 3600) / 60) as f64;
        let mut seconds = (seconds % 60) as f64;

        if config.smooth {
            let millis = f64::from(duration.subsec_millis());
            seconds += millis / 1000.0;
            minutes += seconds / 60.0;
            hours += minutes / 60.0;
        }

        let mode = if config.mode { 24.0 } else { 12.0 };
        let parameters = HashMap::from([
            ("Hours", hours % mode / mode),
            ("Minutes", minutes / 60.0),
            ("Seconds", seconds / 60.0),
        ]);

        for (parameter, arg) in parameters {
            let packet = OscPacket::Message(OscMessage {
                addr: "/avatar/parameters/VRCOSC/Clock/".to_owned() + parameter,
                args: vec![OscType::Float(arg as f32)],
            });

            let msg_buf = rosc::encoder::encode(&packet)?;
            socket.send(&msg_buf)?;
        }

        std::thread::sleep(Duration::from_millis(config.polling));
    }
}
