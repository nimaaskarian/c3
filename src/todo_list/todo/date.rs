use chrono::{Local ,NaiveDate};
use chrono::format::ParseError;
const FORMAT: &str = " %Y-%m-%d";

#[inline]
pub fn parse(date_string: String) -> Result<NaiveDate, ParseError> {
    NaiveDate::parse_from_str(date_string.as_str(), FORMAT)
}

#[inline]
pub fn current() -> NaiveDate {
    NaiveDate::from(Local::now().naive_local())
}

#[inline]
pub fn current_str() -> String {
    current().format(FORMAT).to_string()
}
