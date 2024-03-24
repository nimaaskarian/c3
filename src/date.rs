use chrono::{Local ,NaiveDate};
use chrono::format::ParseError;
const FORMAT: &str = "%Y-%m-%d";

pub type Type = NaiveDate;

#[inline]
pub fn parse(date_string: &String) -> Result<Type, ParseError> {
    NaiveDate::parse_from_str(date_string.as_str(), FORMAT)
}

#[inline]
pub fn current() -> Type {
    NaiveDate::from(Local::now().naive_local())
}

#[inline]
pub fn format(input: Option<Type>) -> String {
    match input {
        Some(date)=> date.format(FORMAT).to_string(),
        None => String::new(),
    }
}
