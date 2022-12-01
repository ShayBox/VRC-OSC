use error_stack::Context;
use std::fmt;

#[derive(Debug)]
pub enum VrcError {
    Io,
    Library,
    None,
    Osc,
    Serde,
    Spotify,
    Toml,
    Url,
}

impl Context for VrcError {}
impl fmt::Display for VrcError {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self {
            VrcError::Io => fmt.write_str("IoError"),
            VrcError::Library => fmt.write_str("LibraryError"),
            VrcError::None => fmt.write_str("None"),
            VrcError::Osc => fmt.write_str("OscError"),
            VrcError::Serde => fmt.write_str("SerdeError"),
            VrcError::Spotify => fmt.write_str("SpotifyError"),
            VrcError::Toml => fmt.write_str("TomlError"),
            VrcError::Url => fmt.write_str("UrlError"),
        }
    }
}
