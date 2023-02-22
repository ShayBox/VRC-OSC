use std::{
    collections::HashMap,
    net::UdpSocket,
    thread::Builder,
    time::{Duration, SystemTime},
};

use abi_stable::{
    export_root_module,
    prefix_type::PrefixTypeTrait,
    sabi_extern_fn,
    sabi_trait::TD_Opaque,
};
use anyhow::Result;
use common::{config::VrcConfig, CommonState_TO, OSCMod, OSCMod_Ref, OscState, StateBox};
use rosc::{OscMessage, OscPacket, OscType};

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

    if config.clock.enable {
        println!("Clock is enabled");
        Builder::new()
            .name("Clock Plugin".to_string())
            .spawn(move || thread_clock(config, &osc).expect("thread_clock"))
            .expect("Clock Plugin failed");
    } else {
        println!("Clock is disabled");
    }

    CommonState_TO::from_value(state, TD_Opaque)
}

fn thread_clock(config: VrcConfig, osc: &UdpSocket) -> Result<()> {
    loop {
        std::thread::sleep(Duration::from_millis(config.clock.polling));

        let duration = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)?;
        let seconds = duration.as_secs();

        let mut hours = (seconds / 3600) as f32;
        let mut minutes = ((seconds % 3600) / 60) as f32;
        let mut seconds = (seconds % 60) as f32;

        if config.clock.smooth {
            let millis = duration.subsec_millis() as f32;
            seconds += millis / 1000.0;
            minutes += seconds / 60.0;
            hours += minutes / 60.0;
        }

        let mode = if config.clock.mode { 24.0 } else { 12.0 };
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
            osc.send_to(&msg_buf, &config.osc.send_addr)?;
        }
    }
}
