use super::error::Error;
use super::Result;
use chrono::naive::NaiveTime;
use serde::{Serialize, Serializer};
use std::str::FromStr;

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
