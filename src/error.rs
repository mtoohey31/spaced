use std::error::Error;
use std::fmt;

#[derive(Debug)]
pub struct ValueError {
    message: Option<String>,
}

impl ValueError {
    pub fn new() -> Self {
        ValueError { message: None }
    }

    pub fn from(message: String) -> Self {
        ValueError {
            message: Some(message),
        }
    }
}

impl Error for ValueError {}

impl fmt::Display for ValueError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.message {
            Some(m) => write!(f, "ValueError: {}", m),
            None => write!(f, "ValueError"),
        }
    }
}
