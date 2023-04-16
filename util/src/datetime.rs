use chrono::{NaiveDate, NaiveDateTime};
use error::{AppResult, Error};

/// Parse string date '%Y-%m-%d' into NaiveDate.
pub fn parse_into_naive_date(raw: &str, err_attribute_name: Option<&str>) -> AppResult<NaiveDate> {
    parse_into_naive_date_format(raw, "%Y-%m-%d", err_attribute_name)
}

/// Parse string date from given format into NaiveDate.
pub fn parse_into_naive_date_format(
    raw: &str,
    fmt: &str,
    err_attribute_name: Option<&str>,
) -> AppResult<NaiveDate> {
    if raw.len() < 10 {
        return Err(Error::BadRequest(format!(
            "invalid_date:{}:{}",
            fmt,
            err_attribute_name.unwrap_or("na")
        )));
    }

    NaiveDate::parse_from_str(&raw[0..10], fmt).map_err(|_| {
        Error::BadRequest(format!(
            "invalid_date:%Y-%m-%d:{}",
            err_attribute_name.unwrap_or("na")
        ))
    })
}

/// Parse string date '%Y-%m-%dT%H:%M:%S%.f' into NaiveDateTime. Time will be treated like UTC
pub fn parse_into_naive_datetime(
    raw: &str,
    err_attribute_name: Option<&str>,
) -> AppResult<NaiveDateTime> {
    parse_into_naive_datetime_format(raw, "%Y-%m-%dT%H:%M:%S%.f", err_attribute_name)
}

/// Special way of formatting that is coming out of the db directly
/// when doing raw sql query
pub fn parse_into_naive_datetime_db(
    raw: &str,
    err_attribute_name: Option<&str>,
) -> AppResult<NaiveDateTime> {
    parse_into_naive_datetime_format(raw, "%Y-%m-%d %H:%M:%S%.f", err_attribute_name)
}

/// Parse string date '%Y-%m-%dT%H:%M:%S%.f' into NaiveDateTime. Time will be treated like UTC
pub fn parse_into_naive_datetime_format(
    raw: &str,
    fmt: &str,
    err_attribute_name: Option<&str>,
) -> AppResult<NaiveDateTime> {
    NaiveDateTime::parse_from_str(raw, fmt).map_err(|_| {
        Error::BadRequest(format!(
            "invalid_datetime:{}:{}",
            fmt,
            err_attribute_name.unwrap_or("na")
        ))
    })
}

#[cfg(test)]
mod test {
    use chrono::{Datelike, Timelike};

    #[test]
    fn test_parse_into_naive_date() {
        let date = super::parse_into_naive_date("2023-04-08T17:28:17.000000", None).unwrap();
        assert_eq!(date.year(), 2023);
        assert_eq!(date.month(), 4);
        assert_eq!(date.day(), 8);
    }

    #[test]
    fn test_parse_into_naive_datetime() {
        let date = super::parse_into_naive_datetime("2023-04-08T17:28:17.000000", None).unwrap();
        assert_eq!(date.year(), 2023);
        assert_eq!(date.month(), 4);
        assert_eq!(date.day(), 8);
        assert_eq!(date.hour(), 17);
        assert_eq!(date.minute(), 28);
        assert_eq!(date.second(), 17);
    }
}
