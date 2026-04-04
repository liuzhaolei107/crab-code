pub type Result<T> = std::result::Result<T, crate::Error>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn result_ok() {
        let r: Result<i32> = Ok(42);
        assert_eq!(r.unwrap(), 42);
    }

    #[test]
    fn result_err() {
        let r: Result<i32> = Err(crate::Error::Other("fail".into()));
        assert!(r.is_err());
    }
}
