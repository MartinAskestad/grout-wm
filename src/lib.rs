#[macro_export]
macro_rules! any {
    ($xs:expr, $x:expr) => {
        $xs.iter().any(|&x| x.0 == $x)
    };
}

#[macro_export]
macro_rules! has_flag {
    ($value:expr, $flag:expr) => {
        ($value & $flag) != 0
    };
}

pub struct Error {
    pub(crate) message: String,
}

impl std::error::Error for Error {}

impl std::fmt::Debug for Error {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut debug = fmt.debug_struct("Error");
        debug.field("message", &self.message).finish()
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::write!(fmt, "{}", self.message)
    }
}

impl std::convert::From<&str> for Error {
    fn from(err: &str) -> Self {
        Error {
            message: String::from(err),
        }
    }
}

impl std::convert::From<windows::core::Error> for Error {
    fn from(err: windows::core::Error) -> Self {
        Error {
            message: err.to_string(),
        }
    }
}

pub type Result<T> = std::result::Result<T, Error>;
