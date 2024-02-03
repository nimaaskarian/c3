use chrono::NaiveDate;
use chrono::Duration;
use scanf::sscanf;

mod date;

#[derive(Debug, PartialEq, Clone, Default)]
pub struct Schedule {
    day: i64,
    done_date: Option<NaiveDate>,
    pub is_scheduled: bool,
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
        let mut is_scheduled = true;
        match input {
            _ if sscanf!(input.to_string().as_str(), "D{}({})", day, date_string).is_ok() => {
            }
            _ => {
                is_scheduled = false
            },
        };
        let last_done = match date::parse(&date_string) {
            Ok(value) => Some(value),
            Err(_) => None
        };

        Schedule {
            day,
            done_date: last_done,
            is_scheduled 
        }
    }
}

impl Into<String> for &Schedule {
    fn into(self) -> String {
        if self.is_scheduled {
            let date_str = date::format(self.done_date);
            match self.day {
                any => format!(" [D{any}({date_str})]"),
            }
        } else {
            String::new()
        }
    }
}

impl Schedule {
    pub fn new() -> Self {
        Schedule {
            day: 0,
            done_date: None,
            is_scheduled: false, 
        }
    }

    fn last_save (&self) -> Duration {
        if let Some(done_date) = self.done_date {
            date::current() - done_date
        } else {
            Duration::zero()
        }
    }

    pub fn display(&self) -> String{
        if !self.is_scheduled {
            return String::new();
        }
        let inner_str = match self.last_save().num_days() {
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
        self.done_date = None
    }

    pub fn current_date(&mut self) {
        self.done_date = Some(date::current())
    }

    pub fn toggle(&mut self) {
        self.is_scheduled = !self.is_scheduled;
    }

    pub fn enable(&mut self) {
        self.is_scheduled = true;
    }

    pub fn match_message(input: &mut String) -> Self {
        let ScheduleRest { schedule, rest } = ScheduleRest::from(input.clone());
        *input = rest;
        schedule
    }

    pub fn should_undone(&self) -> bool {
        return self.is_scheduled && self.last_save().num_days() >= self.day 
    }
}
