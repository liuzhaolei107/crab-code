use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("config error: {0}")]
    Config(String),

    #[error("API error: {0}")]
    Api(String),

    #[error("auth error: {0}")]
    Auth(String),

    #[error("tool error: {0}")]
    Tool(String),

    #[error("permission error: {0}")]
    Permission(String),

    #[error("{0}")]
    Other(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn io_error_from_conversion() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let err: Error = io_err.into();
        assert!(matches!(err, Error::Io(_)));
        assert!(err.to_string().contains("file not found"));
    }

    #[test]
    fn config_error_display() {
        let err = Error::Config("bad value".into());
        assert_eq!(err.to_string(), "config error: bad value");
    }

    #[test]
    fn api_error_display() {
        let err = Error::Api("rate limited".into());
        assert_eq!(err.to_string(), "API error: rate limited");
    }

    #[test]
    fn auth_error_display() {
        let err = Error::Auth("token expired".into());
        assert_eq!(err.to_string(), "auth error: token expired");
    }

    #[test]
    fn tool_error_display() {
        let err = Error::Tool("bash failed".into());
        assert_eq!(err.to_string(), "tool error: bash failed");
    }

    #[test]
    fn permission_error_display() {
        let err = Error::Permission("denied".into());
        assert_eq!(err.to_string(), "permission error: denied");
    }

    #[test]
    fn other_error_display() {
        let err = Error::Other("something went wrong".into());
        assert_eq!(err.to_string(), "something went wrong");
    }

    #[test]
    fn error_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<Error>();
    }
}
