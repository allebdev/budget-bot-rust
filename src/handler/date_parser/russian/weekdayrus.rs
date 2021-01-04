use std::str::FromStr;

use chrono::Weekday;

pub struct WeekdayRus(Weekday);

impl From<WeekdayRus> for Weekday {
    fn from(wd: WeekdayRus) -> Self {
        wd.0
    }
}

impl FromStr for WeekdayRus {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_ref() {
            "пн" | "понедельник" => Ok(WeekdayRus(Weekday::Mon)),
            "вт" | "вторник" => Ok(WeekdayRus(Weekday::Tue)),
            "ср" | "среда" | "среду" => Ok(WeekdayRus(Weekday::Wed)),
            "чт" | "четверг" => Ok(WeekdayRus(Weekday::Thu)),
            "пт" | "пятница" | "пятницу" => Ok(WeekdayRus(Weekday::Fri)),
            "сб" | "суббота" | "субботу" => Ok(WeekdayRus(Weekday::Sat)),
            "вс" | "воскресенье" => Ok(WeekdayRus(Weekday::Sun)),
            _ => Err(()),
        }
    }
}
