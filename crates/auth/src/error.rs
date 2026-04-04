#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("auth error: {message}")]
    Auth { message: String },

    #[error(transparent)]
    Common(#[from] crab_common::Error),
}

impl From<AuthError> for crab_common::Error {
    fn from(err: AuthError) -> Self {
        match err {
            AuthError::Auth { message } => Self::Auth(message),
            AuthError::Common(e) => e,
        }
    }
}
