use chrono::Duration;
use scanf::sscanf;

use crate::date;

#[derive(Debug, PartialEq, Clone, Default)]
pub enum ScheduleType {
    Scheduled,
    Reminder,
    #[default]
    None,
}

type Type = ScheduleType;
#[derive(Debug, PartialEq, Clone, Default)]
pub struct Schedule {
    day: i64,
    date: Option<date::Type>,
    _type: Type,
    pub last_type: Type,
}

struct ScheduleRest {
    schedule: Schedule,
    rest: String,
}

impl From<String> for ScheduleRest {

    fn from(input:String) -> ScheduleRest {
        let mut schedule_str = String::new();
        let mut rest = String::new();
        if sscanf!(input.as_str(), "{}[{}]", rest, schedule_str).is_ok() {
        } else {
            rest = input;
        }

        let schedule = Schedule::from(schedule_str);
        ScheduleRest {
            schedule,
            rest
        }
    }
}

impl<T> From<T> for Schedule 
where
T: ToString,
{
    fn from(input:T) -> Schedule {
        let mut day = 0;
        let mut date_string = String::new();
        let _type = match input {
            _ if sscanf!(input.to_string().as_str(), "D{}({})", day, date_string).is_ok() => {
                Type::Scheduled
            }
            _ if sscanf!(input.to_string().as_str(), "R({})", date_string).is_ok() => {
                Type::Reminder
            }
            _ => {
                Type::None
            },
        };
        let date = match date::parse(&date_string) {
            Ok(value) => Some(value),
            Err(_) => None
        };

        Schedule {
            day,
            date,
            _type,
            last_type: Type::default(),
        }
    }
}

impl Into<String> for &Schedule {
    fn into(self) -> String {
        let date_str = date::format(self.date);

        match self._type {
            Type::Reminder => format!(" [R({date_str})]"),
            Type::Scheduled =>  format!(" [D{}({date_str})]", self.day),
            Type::None => String::new(),
        }
    }
}

impl Schedule {
    pub fn new() -> Self {
        Schedule {
            day: 0,
            date: None,
            _type: Type::default(),
            last_type: Type::default(),
        }
    }

    #[inline(always)]
    fn current_date_diff_days(&self) -> i64 {
        date::diff_days(Some(date::current()), self.date)
    }

    #[inline(always)]
    fn date_diff_days(&self) -> i64 {
        date::diff_days(self.date, Some(date::current()))
    }

    #[inline(always)]
    fn display_reminder(&self) -> String {
        let date_str = date::format(self.date);
        match self.date_diff_days() {
            any if any < 0 => format!(" (Reminder for {} [{} days ago])", date_str,-1*any),
            0 => format!(" (Reminder for today [{}])", date_str),
            1 => format!(" (Reminder for tomorrow [{}])", date_str),
            any => format!(" (Reminder for {date_str} [{any} days])"),
        }
    }

    #[inline(always)]
    fn display_scheduled(&self) -> String {
        let inner_str = match self.current_date_diff_days() {
            0 => String::new(),
            1 => String::from(", last done yesterday"),
            any => format!(", last done {} days ago", any)
        };
        match self.day {
            1 =>format!(" (Daily{inner_str})"),
            7 => format!(" (Weekly{inner_str})"),
            day if day%7 == 0 => format!(" (Each {} weeks{inner_str})", day/7),
            day =>format!(" (Each {day} days{inner_str})"),
        }
    }

    pub fn display(&self) -> String{
        match self._type {
            Type::Reminder => self.display_reminder(),
            Type::Scheduled => self.display_scheduled(),
            Type::None => String::new(),
        }
    }

    pub fn add_days_to_done_date(&mut self, days:i64) {
        if let Some(date) = self.date {
            if days <= self.current_date_diff_days() {
                self.date = Some(date+Duration::days(days));
            }
        }
    }

    pub fn set_daily(&mut self) {
        self.set_day(1);
    }

    pub fn set_day(&mut self, day: i64) {
        self.day = day;
    }

    pub fn set_weekly(&mut self) {
        self.set_day(7)
    }


    pub fn none_date(&mut self) {
        self.date = None
    }

    pub fn current_date(&mut self) {
        self.date = Some(date::current())
    }

    pub fn toggle(&mut self) {
        match self._type.clone() {
            Type::None => self._type = self.last_type.clone(),
            any => {
                self.last_type = any;
                self._type = Type::None;
            }
        };
    }

    pub fn enable_schedule(&mut self) {
        self._type = Type::Scheduled;
    }

    pub fn enable_reminder(&mut self, date: date::Type){
        self._type = Type::Reminder;
        self.date = Some(date);
    }


    pub fn match_message(input: &mut String) -> Self {
        let ScheduleRest { schedule, rest } = ScheduleRest::from(input.clone());
        *input = rest;
        schedule
    }

    #[inline(always)]
    pub fn is_reminder(&self) -> bool {
        self._type == Type::Reminder
    }

    #[inline(always)]
    fn reminder_should_undone(&self) -> bool {
        self.date == Some(date::current())
    }

    pub fn should_undone(&self) -> bool {
        match self._type {
            Type::Reminder => self.reminder_should_undone(),
            Type::Scheduled => self.current_date_diff_days() >= self.day,
            Type::None => false,
        }
    }

    pub fn should_done(&self) -> bool {
        match self._type {
            Type::Reminder => !self.reminder_should_undone(),
            _ => false
        }
    }
}
