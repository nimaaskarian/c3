use std::str::FromStr;

use crate::date;

#[derive(Eq, Debug, PartialEq, Clone, Default)]
pub enum ScheduleMode {
    #[default]
    Scheduled,
    Reminder,
}

#[derive(Eq, Debug, PartialEq, Clone, Default)]
pub struct Schedule {
    day: i64,
    saved_date: Option<date::Type>,
    mode: ScheduleMode,
}

#[derive(Default)]
enum State {
    #[default]
    Type,
    Days,
    PreDate,
    Date,
}

pub struct NotScheduled;

impl FromStr for Schedule {
    type Err = NotScheduled;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut date_string = String::new();
        let mut state = State::default();
        let mut mode = None;
        let mut day_str = String::new();

        for c in s.to_string().chars() {
            match state {
                State::Type => {
                    if c == 'D' {
                        mode = Some(ScheduleMode::Scheduled);
                        state = State::Days;
                    } else if c == 'R' {
                        mode = Some(ScheduleMode::Reminder);
                        state = State::PreDate;
                    } else {
                        break;
                    }
                }
                State::Days => {
                    if c.is_ascii_digit() {
                        day_str.push(c);
                    } else if c == '(' {
                        state = State::Date;
                    }
                }
                State::PreDate => {
                    if c == '(' {
                        state = State::Date;
                    }
                }
                State::Date => {
                    if c == ')' {
                        break;
                    } else {
                        date_string.push(c)
                    }
                }
            }
        }

        let day: i64 = day_str.parse().unwrap_or(0);

        let date = match date::parse(&date_string) {
            Ok(value) => Some(value),
            Err(_) => None,
        };
        if let Some(mode) = mode {
            Ok(Schedule { day, saved_date: date, mode })
        } else {
            Err(Self::Err {})
        }
    }
}

impl From<&Schedule> for String {
    fn from(schedule: &Schedule) -> String {
        let date_str = date::format(schedule.saved_date);

        match schedule.mode {
            ScheduleMode::Reminder => format!(" [R({date_str})]"),
            ScheduleMode::Scheduled => format!(" [D{}({date_str})]", schedule.day),
        }
    }
}

impl Schedule {
    pub fn new(day: i64) -> Self {
        Schedule {
            day,
            ..Default::default()
        }
    }

    pub fn new_reminder(date: date::Type) -> Self {
        Self {
            mode: ScheduleMode::Reminder,
            saved_date: Some(date),
            ..Default::default()
        }
    }

    #[inline(always)]
    fn current_minus_saved_date(&self) -> i64 {
        date::diff_days(Some(date::current()), self.saved_date)
    }

    #[inline(always)]
    fn saved_minus_current_date(&self) -> i64 {
        date::diff_days(self.saved_date, Some(date::current()))
    }

    pub fn days(&self) -> i64 {
        match self.mode {
            ScheduleMode::Scheduled => self.day,
            ScheduleMode::Reminder => 1,
        }
    }

    pub fn days_diff(&self) -> i64 {
        match self.mode {
            ScheduleMode::Scheduled => self.current_minus_saved_date(),
            ScheduleMode::Reminder => self.saved_minus_current_date(),
        }
    }

    #[inline(always)]
    fn display_reminder(&self) -> String {
        let date_str = date::display(self.saved_date);
        match self.saved_minus_current_date() {
            any if any < 0 => format!(" (Reminder for {} [{} days ago])", date_str, -any),
            0 => format!(" (Reminder for today [{}])", date_str),
            1 => format!(" (Reminder for tomorrow [{}])", date_str),
            any => format!(" (Reminder for {date_str} [{any} days])"),
        }
    }

    #[inline(always)]
    fn display_scheduled(&self) -> String {
        let inner_str = match self.current_minus_saved_date() {
            ..=0 => String::new(),
            1 => String::from(", last done yesterday"),
            7 => String::from(", last done a week ago"),
            any if any % 7 == 0 => format!(", last done {} weeks ago", any / 7),
            any => format!(", last done {} days ago", any),
        };
        match self.day {
            1 => format!(" (Daily{inner_str})"),
            7 => format!(" (Weekly{inner_str})"),
            day if day % 7 == 0 => format!(" (Each {} weeks{inner_str})", day / 7),
            day => format!(" (Each {day} days{inner_str})"),
        }
    }

    pub fn display(&self) -> String {
        match self.mode {
            ScheduleMode::Reminder => self.display_reminder(),
            ScheduleMode::Scheduled => self.display_scheduled(),
        }
    }

    pub fn add_days_to_date(&mut self, days: i64) {
        if let Some(date) = self.saved_date {
            if days > 0
                && self.mode == ScheduleMode::Scheduled
                && self.current_minus_saved_date() <= 0
            {
                return;
            }
            self.saved_date = Some(date::add_days(date, days))
        }
    }

    pub fn set_day(&mut self, day: i64) {
        self.day = day;
    }

    pub fn set_current_date(&mut self) {
        if self.mode == ScheduleMode::Scheduled {
            self.saved_date = Some(date::current())
        }
    }

    #[inline(always)]
    pub fn is_reminder(&self) -> bool {
        self.mode == ScheduleMode::Reminder
    }

    pub fn date_should_be_done(&self) -> bool {
        match self.mode {
            ScheduleMode::Reminder => self.saved_date != Some(date::current()),
            ScheduleMode::Scheduled => self.current_minus_saved_date() < self.day,
        }
    }
}
