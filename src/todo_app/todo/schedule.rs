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

#[derive(Default)]
enum State {
    #[default]
    Type,
    Days,
    PreDate,
    Date,
}

impl<T> From<T> for Schedule 
where
T: ToString,
{
    fn from(input:T) -> Schedule {
        let mut date_string = String::new();
        let mut state = State::default();
        let mut _type = Type::None;
        let mut day_str = String::new();

        for c in input.to_string().chars() {
            match state {
                State::Type => {
                    if c == 'D' {
                        _type = Type::Scheduled;
                        state = State::Days;
                    } else if c == 'R' {
                        _type = Type::Reminder;
                        state = State::PreDate;
                    } else {
                        break;
                    }
                }
                State::Days => {
                    if c.is_digit(10) {
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

        let day:i64 = day_str.parse().unwrap_or(0);

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
        let date_str = date::display(self.date);
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
            ..=0 => String::new(),
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

    pub fn add_days_to_date(&mut self, days:i64) {
        if let Some(date) = self.date {
            if days > 0 && self._type == Type::Scheduled && self.current_date_diff_days() <= 0 {
                return
            }
            self.date = Some(date::add_days(date, days))
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

    #[inline]
    pub fn is_weekly(&self) -> bool {
        self.is_scheduled() && self.day == 7
    }

    #[inline]
    pub fn is_daily(&self) -> bool {
        self.is_scheduled() && self.day == 1
    }

    pub fn none_date(&mut self) {
        self.date = None
    }

    pub fn set_current_date(&mut self) {
        if self._type == Type::Scheduled {
            self.date = Some(date::current())
        }
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

    #[inline(always)]
    pub fn is_reminder(&self) -> bool {
        self._type == Type::Reminder
    }

    #[inline(always)]
    pub fn is_scheduled(&self) -> bool {
        self._type == Type::Scheduled
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
