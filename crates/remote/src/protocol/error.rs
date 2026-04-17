//! Error codes used in crab-proto [`JsonRpcError`] payloads.
//!
//! The first four codes mirror JSON-RPC 2.0 standard error codes exactly
//! (same numbers any well-formed client already handles). Subsequent
//! codes are crab-proto's own, allocated in the vendor-range JSON-RPC
//! reserves (-32000 down).
//!
//! [`JsonRpcError`]: super::envelope::JsonRpcError

/// Canonical crab-proto error codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum ErrorCode {
    // JSON-RPC 2.0 standard codes
    ParseError = -32700,
    InvalidRequest = -32600,
    MethodNotFound = -32601,
    InvalidParams = -32602,
    InternalError = -32603,

    // crab-proto vendor codes
    /// Client tried to call a session method without first attaching.
    NotAttached = -32000,
    /// Auth handshake failed (invalid JWT, missing token, etc.).
    Unauthorized = -32001,
    /// Protocol version incompatible with server.
    UnsupportedVersion = -32002,
    /// Session referenced by id does not exist on the server.
    SessionNotFound = -32003,
    /// Server is shutting down; new work refused.
    Shutdown = -32004,
}

impl From<ErrorCode> for i32 {
    fn from(code: ErrorCode) -> Self {
        code as Self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standard_codes_match_json_rpc_spec() {
        assert_eq!(i32::from(ErrorCode::ParseError), -32700);
        assert_eq!(i32::from(ErrorCode::InvalidRequest), -32600);
        assert_eq!(i32::from(ErrorCode::MethodNotFound), -32601);
        assert_eq!(i32::from(ErrorCode::InvalidParams), -32602);
        assert_eq!(i32::from(ErrorCode::InternalError), -32603);
    }

    #[test]
    fn vendor_codes_are_in_reserved_range() {
        for c in [
            ErrorCode::NotAttached,
            ErrorCode::Unauthorized,
            ErrorCode::UnsupportedVersion,
            ErrorCode::SessionNotFound,
            ErrorCode::Shutdown,
        ] {
            let n = i32::from(c);
            assert!(
                (-32099..=-32000).contains(&n),
                "code {n:?} must be in JSON-RPC server-reserved range -32099..=-32000"
            );
        }
    }
}
