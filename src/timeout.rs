//! Types representing Golem Task's timeout values
use super::{error::Error, Result};
use chrono::naive::NaiveTime;
use serde::{Serialize, Serializer};
use std::str::FromStr;
use std::fmt;

/// Wrapper type for [`NaiveTime`]
///
/// `Timeout` can be generated from `str` only, and accepts format `%H:%M:%S`. Note
/// that zero timeout is treated as an error: [`Error::ZeroTimeoutError`].
///
/// [`NaiveTime`]: https://docs.rs/chrono/0.4.7/chrono/naive/struct.NaiveTime.html
/// [`Error::ZeroTimeoutError`]: ../error/enum.Error.html#variant.ZeroTimeoutError
///
/// # Example:
/// ```rust
/// use gwasm_api::timeout::Timeout;
/// use std::str::FromStr;
/// use chrono::naive::NaiveTime;
///
/// assert!(Timeout::from_str("00:00:10").is_ok());
/// assert!(Timeout::from_str("10").is_err());
/// assert!(Timeout::from_str("00:00:00").is_err());
/// ```
#[derive(Debug, Copy, Clone, PartialEq, Serialize)]
pub struct Timeout(#[serde(serialize_with = "serialize_naive_time")] NaiveTime);

impl FromStr for Timeout {
    type Err = Error;

    fn from_str(value: &str) -> Result<Self> {
        let timeout = NaiveTime::parse_from_str(value, "%H:%M:%S")?;
        if timeout == NaiveTime::from_hms(0, 0, 0) {
            Err(Error::ZeroTimeoutError)
        } else {
            Ok(Self(timeout))
        }
    }
}

impl fmt::Display for Timeout {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0.format("%H:%M:%S").to_string())
    }
}

fn serialize_naive_time<S: Serializer>(
    timeout: &NaiveTime,
    s: S,
) -> std::result::Result<S::Ok, S::Error> {
    s.serialize_str(&timeout.format("%H:%M:%S").to_string())
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn valid_input() {
        assert_eq!(
            Timeout::from_str("00:00:10").unwrap(),
            Timeout(NaiveTime::from_hms(0, 0, 10))
        );
        assert_eq!(
            Timeout::from_str("00:10:00").unwrap(),
            Timeout(NaiveTime::from_hms(0, 10, 0))
        );
        assert_eq!(
            Timeout::from_str("10:00:00").unwrap(),
            Timeout(NaiveTime::from_hms(10, 0, 0))
        );
        assert_eq!(
            Timeout::from_str("23:59:59").unwrap(),
            Timeout(NaiveTime::from_hms(23, 59, 59))
        );
    }

    #[test]
    fn invalid_input() {
        assert!(Timeout::from_str("10").is_err());
        assert!(Timeout::from_str("10:00").is_err());
        assert!(Timeout::from_str("").is_err());
        assert!(Timeout::from_str("24:00:00").is_err());
        assert!(Timeout::from_str("00:00:00").is_err());
    }
}
