fn main() -> Result<(), Box<dyn std::error::Error>> {
    if cfg!(target_os = "windows") {
        winres::WindowsResource::new()
            .set_icon("Icon.ico")
            .compile()?;
    }

    Ok(())
}
