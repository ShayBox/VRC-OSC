use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    if cfg!(target_os = "windows") {
        winres::WindowsResource::new()
            .set_icon("VRC-OSC.ico")
            .compile()?;
    }

    Ok(())
}
