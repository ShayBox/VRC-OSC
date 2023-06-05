use std::{
    collections::HashMap,
    net::UdpSocket,
    time::{Duration, SystemTime},
};

use anyhow::Result;
use rosc::{OscMessage, OscPacket, OscType};

use crate::config::ClockConfig;

mod config;

#[no_mangle]
#[tokio::main(flavor = "current_thread")]
async fn load(socket: UdpSocket) -> Result<()> {
    let config = ClockConfig::load()?;

    loop {
        let duration = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)?;
        let seconds = duration.as_secs();

        let mut hours = (seconds / 3600) as f32;
        let mut minutes = ((seconds % 3600) / 60) as f32;
        let mut seconds = (seconds % 60) as f32;

        if config.smooth {
            let millis = duration.subsec_millis() as f32;
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
                args: vec![OscType::Float(arg)],
            });

            let msg_buf = rosc::encoder::encode(&packet)?;
            socket.send(&msg_buf)?;
        }

        std::thread::sleep(Duration::from_millis(config.polling));
    }
}
