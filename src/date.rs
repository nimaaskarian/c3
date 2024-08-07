use chrono::format::ParseError;
use chrono::{Duration, Local, NaiveDate};
const FORMAT: &str = "%Y-%m-%d";

pub type Type = NaiveDate;

#[inline]
pub fn parse(date_string: &str) -> Result<Type, ParseError> {
    NaiveDate::parse_from_str(date_string, FORMAT)
}

#[inline(always)]
pub fn parse_user_input(date_string: &str) -> Result<Type, ParseError> {
    parse(date_string)
}

#[inline]
pub fn current() -> Type {
    NaiveDate::from(Local::now().naive_local())
}

#[inline]
pub fn format(input: Option<Type>) -> String {
    match input {
        Some(date) => date.format(FORMAT).to_string(),
        None => String::new(),
    }
}

#[inline(always)]
pub fn display(input: Option<Type>) -> String {
    format(input)
}

#[inline]
pub fn diff_days(first: Option<Type>, next: Option<Type>) -> i64 {
    match (first, next) {
        (Some(first), Some(next)) => (first - next).num_days(),
        _ => 0,
    }
}

pub fn add_days(date: Type, days: i64) -> Type {
    date + Duration::days(days)
}
