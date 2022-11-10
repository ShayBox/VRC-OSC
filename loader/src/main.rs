use common::Error;
use error_stack::Result;
use std::{thread, time::Duration};

fn main() -> Result<(), Error> {
    vrc_osc::load_plugins()?;

    loop {
        thread::sleep(Duration::from_secs(u64::MAX))
    }
}
