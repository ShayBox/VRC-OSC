use error_stack::Context;
use std::fmt;

#[derive(Debug)]
pub enum VRCError {
    IOError,
    LibraryError,
    None,
    OscError,
    SerdeError,
    TOMLError,
}

impl Context for VRCError {}
impl fmt::Display for VRCError {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self {
            VRCError::IOError => fmt.write_str("IOError"),
            VRCError::LibraryError => fmt.write_str("LibraryError"),
            VRCError::None => fmt.write_str("None"),
            VRCError::OscError => fmt.write_str("OscError"),
            VRCError::SerdeError => fmt.write_str("SerdeError"),
            VRCError::TOMLError => fmt.write_str("TOMLError"),
        }
    }
}
