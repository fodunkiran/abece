//! The names of the months and the days of the week, and dates written
//! out: "25 Temmuz 2026 Cumartesi", "20 Temmuz – 31 Ağustos 2026".
//!
//! A [`Date`] is a plain year, month and day; it knows its weekday and its
//! Turkish spellings and nothing about time zones or arithmetic. With the
//! `jiff` feature it converts from and to `jiff::civil::Date`.

use alloc::string::String;
use core::fmt;

/// A month of the year.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[allow(missing_docs)]
pub enum Month {
    January = 1,
    February = 2,
    March = 3,
    April = 4,
    May = 5,
    June = 6,
    July = 7,
    August = 8,
    September = 9,
    October = 10,
    November = 11,
    December = 12,
}

impl Month {
    /// The twelve months in order.
    pub const ALL: [Month; 12] = [
        Month::January,
        Month::February,
        Month::March,
        Month::April,
        Month::May,
        Month::June,
        Month::July,
        Month::August,
        Month::September,
        Month::October,
        Month::November,
        Month::December,
    ];

    /// The month numbered `number`, `1` to `12`.
    ///
    /// ```
    /// use abece::date::Month;
    ///
    /// assert_eq!(Month::from_number(7), Some(Month::July));
    /// assert_eq!(Month::from_number(13), None);
    /// assert_eq!(Month::from_number(0), None);
    /// ```
    #[must_use]
    pub const fn from_number(number: u8) -> Option<Self> {
        Some(match number {
            1 => Month::January,
            2 => Month::February,
            3 => Month::March,
            4 => Month::April,
            5 => Month::May,
            6 => Month::June,
            7 => Month::July,
            8 => Month::August,
            9 => Month::September,
            10 => Month::October,
            11 => Month::November,
            12 => Month::December,
            _ => return None,
        })
    }

    /// The month's number, `1` to `12`.
    ///
    /// ```
    /// assert_eq!(abece::date::Month::October.number(), 10);
    /// ```
    #[must_use]
    pub const fn number(self) -> u8 {
        self as u8
    }

    /// The month's name: "Ocak", "Şubat", "Mart", "Nisan", "Mayıs",
    /// "Haziran", "Temmuz", "Ağustos", "Eylül", "Ekim", "Kasım", "Aralık".
    ///
    /// ```
    /// assert_eq!(abece::date::Month::July.name(), "Temmuz");
    /// ```
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Month::January => "Ocak",
            Month::February => "Şubat",
            Month::March => "Mart",
            Month::April => "Nisan",
            Month::May => "Mayıs",
            Month::June => "Haziran",
            Month::July => "Temmuz",
            Month::August => "Ağustos",
            Month::September => "Eylül",
            Month::October => "Ekim",
            Month::November => "Kasım",
            Month::December => "Aralık",
        }
    }

    /// The name's first three letters: "Oca", "Şub", "Mar", "Nis", "May",
    /// "Haz", "Tem", "Ağu", "Eyl", "Eki", "Kas", "Ara".
    ///
    /// ```
    /// assert_eq!(abece::date::Month::August.short_name(), "Ağu");
    /// ```
    #[must_use]
    pub const fn short_name(self) -> &'static str {
        match self {
            Month::January => "Oca",
            Month::February => "Şub",
            Month::March => "Mar",
            Month::April => "Nis",
            Month::May => "May",
            Month::June => "Haz",
            Month::July => "Tem",
            Month::August => "Ağu",
            Month::September => "Eyl",
            Month::October => "Eki",
            Month::November => "Kas",
            Month::December => "Ara",
        }
    }

    /// The number of days the month has in `year`.
    ///
    /// ```
    /// use abece::date::Month;
    ///
    /// assert_eq!(Month::February.days(2024), 29);
    /// assert_eq!(Month::February.days(2026), 28);
    /// assert_eq!(Month::February.days(1900), 28);
    /// assert_eq!(Month::February.days(2000), 29);
    /// assert_eq!(Month::April.days(2026), 30);
    /// ```
    #[must_use]
    pub const fn days(self, year: i32) -> u8 {
        match self {
            Month::January
            | Month::March
            | Month::May
            | Month::July
            | Month::August
            | Month::October
            | Month::December => 31,
            Month::April | Month::June | Month::September | Month::November => 30,
            Month::February => {
                if is_leap_year(year) {
                    29
                } else {
                    28
                }
            }
        }
    }
}

/// Whether `year` is a leap year of the Gregorian calendar.
///
/// ```
/// use abece::date::is_leap_year;
///
/// assert!(is_leap_year(2024) && is_leap_year(2000));
/// assert!(!is_leap_year(2026) && !is_leap_year(1900));
/// ```
#[must_use]
pub const fn is_leap_year(year: i32) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}

/// A day of the week.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[allow(missing_docs)]
pub enum Weekday {
    Monday = 1,
    Tuesday = 2,
    Wednesday = 3,
    Thursday = 4,
    Friday = 5,
    Saturday = 6,
    Sunday = 7,
}

impl Weekday {
    /// The seven days, Monday first.
    pub const ALL: [Weekday; 7] = [
        Weekday::Monday,
        Weekday::Tuesday,
        Weekday::Wednesday,
        Weekday::Thursday,
        Weekday::Friday,
        Weekday::Saturday,
        Weekday::Sunday,
    ];

    /// The day numbered `number`, Monday being `1` and Sunday `7` (ISO 8601).
    ///
    /// ```
    /// use abece::date::Weekday;
    ///
    /// assert_eq!(Weekday::from_number(1), Some(Weekday::Monday));
    /// assert_eq!(Weekday::from_number(7), Some(Weekday::Sunday));
    /// assert_eq!(Weekday::from_number(0), None);
    /// ```
    #[must_use]
    pub const fn from_number(number: u8) -> Option<Self> {
        Some(match number {
            1 => Weekday::Monday,
            2 => Weekday::Tuesday,
            3 => Weekday::Wednesday,
            4 => Weekday::Thursday,
            5 => Weekday::Friday,
            6 => Weekday::Saturday,
            7 => Weekday::Sunday,
            _ => return None,
        })
    }

    /// The day's number, Monday being `1` and Sunday `7`.
    ///
    /// ```
    /// assert_eq!(abece::date::Weekday::Saturday.number(), 6);
    /// ```
    #[must_use]
    pub const fn number(self) -> u8 {
        self as u8
    }

    /// The day's name: "Pazartesi", "Salı", "Çarşamba", "Perşembe", "Cuma",
    /// "Cumartesi", "Pazar".
    ///
    /// ```
    /// assert_eq!(abece::date::Weekday::Wednesday.name(), "Çarşamba");
    /// ```
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Weekday::Monday => "Pazartesi",
            Weekday::Tuesday => "Salı",
            Weekday::Wednesday => "Çarşamba",
            Weekday::Thursday => "Perşembe",
            Weekday::Friday => "Cuma",
            Weekday::Saturday => "Cumartesi",
            Weekday::Sunday => "Pazar",
        }
    }

    /// The day's short name, as a calendar's column heading: "Pzt", "Sal",
    /// "Çar", "Per", "Cum", "Cmt", "Paz".
    ///
    /// ```
    /// assert_eq!(abece::date::Weekday::Saturday.short_name(), "Cmt");
    /// ```
    #[must_use]
    pub const fn short_name(self) -> &'static str {
        match self {
            Weekday::Monday => "Pzt",
            Weekday::Tuesday => "Sal",
            Weekday::Wednesday => "Çar",
            Weekday::Thursday => "Per",
            Weekday::Friday => "Cum",
            Weekday::Saturday => "Cmt",
            Weekday::Sunday => "Paz",
        }
    }
}

/// A calendar date: a year, a month and a day of the proleptic Gregorian
/// calendar, valid by construction.
///
/// ```
/// use abece::date::{Date, Month, Weekday};
///
/// let day = Date::new(2026, 7, 25).unwrap();
/// assert_eq!(day.month(), Month::July);
/// assert_eq!(day.weekday(), Weekday::Saturday);
/// assert_eq!(day.to_string(), "25 Temmuz 2026");
/// assert_eq!(Date::new(2026, 2, 29), None);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Date {
    year: i32,
    month: Month,
    day: u8,
}

/// How a [`Date`] is written out.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum DateStyle {
    /// "25.07.2026".
    Numeric,
    /// "25 Temmuz 2026".
    Long,
    /// "25 Temmuz 2026 Cumartesi".
    LongWithWeekday,
    /// "25 Temmuz 2026, Cumartesi".
    LongCommaWeekday,
    /// "25 Temmuz".
    DayMonth,
    /// "25 Tem".
    ShortDayMonth,
    /// "Temmuz 2026".
    MonthYear,
}

impl Date {
    /// The date `year`-`month`-`day`, or `None` when there is no such day.
    ///
    /// ```
    /// use abece::date::Date;
    ///
    /// assert!(Date::new(2024, 2, 29).is_some());
    /// assert!(Date::new(2026, 2, 29).is_none());
    /// assert!(Date::new(2026, 13, 1).is_none());
    /// assert!(Date::new(2026, 4, 31).is_none());
    /// assert!(Date::new(2026, 1, 0).is_none());
    /// ```
    #[must_use]
    pub const fn new(year: i32, month: u8, day: u8) -> Option<Self> {
        match Month::from_number(month) {
            Some(month) => Self::from_parts(year, month, day),
            None => None,
        }
    }

    /// The date `year`-`month`-`day` with the month as a [`Month`], or
    /// `None` when there is no such day.
    ///
    /// ```
    /// use abece::date::{Date, Month};
    ///
    /// assert!(Date::from_parts(2026, Month::July, 25).is_some());
    /// assert!(Date::from_parts(2026, Month::June, 31).is_none());
    /// ```
    #[must_use]
    pub const fn from_parts(year: i32, month: Month, day: u8) -> Option<Self> {
        if day == 0 || day > month.days(year) {
            return None;
        }
        Some(Self { year, month, day })
    }

    /// The year.
    ///
    /// ```
    /// assert_eq!(abece::date::Date::new(2026, 7, 25).unwrap().year(), 2026);
    /// ```
    #[must_use]
    pub const fn year(self) -> i32 {
        self.year
    }

    /// The month.
    ///
    /// ```
    /// assert_eq!(abece::date::Date::new(2026, 7, 25).unwrap().month(), abece::date::Month::July);
    /// ```
    #[must_use]
    pub const fn month(self) -> Month {
        self.month
    }

    /// The day of the month, `1` to `31`.
    ///
    /// ```
    /// assert_eq!(abece::date::Date::new(2026, 7, 25).unwrap().day(), 25);
    /// ```
    #[must_use]
    pub const fn day(self) -> u8 {
        self.day
    }

    /// The day of the week.
    ///
    /// ```
    /// use abece::date::{Date, Weekday};
    ///
    /// assert_eq!(Date::new(2026, 7, 25).unwrap().weekday(), Weekday::Saturday);
    /// assert_eq!(Date::new(1970, 1, 1).unwrap().weekday(), Weekday::Thursday);
    /// assert_eq!(Date::new(2000, 2, 29).unwrap().weekday(), Weekday::Tuesday);
    /// ```
    #[must_use]
    pub const fn weekday(self) -> Weekday {
        // 1970-01-01 was a Thursday.
        let days = days_from_civil(self.year, self.month.number(), self.day);
        match days.rem_euclid(7) {
            0 => Weekday::Thursday,
            1 => Weekday::Friday,
            2 => Weekday::Saturday,
            3 => Weekday::Sunday,
            4 => Weekday::Monday,
            5 => Weekday::Tuesday,
            _ => Weekday::Wednesday,
        }
    }

    /// The date written out in `style`.
    ///
    /// ```
    /// use abece::date::{Date, DateStyle};
    ///
    /// let day = Date::new(2026, 10, 22).unwrap();
    /// assert_eq!(day.format(DateStyle::Numeric), "22.10.2026");
    /// assert_eq!(day.format(DateStyle::Long), "22 Ekim 2026");
    /// assert_eq!(day.format(DateStyle::LongWithWeekday), "22 Ekim 2026 Perşembe");
    /// assert_eq!(day.format(DateStyle::LongCommaWeekday), "22 Ekim 2026, Perşembe");
    /// assert_eq!(day.format(DateStyle::DayMonth), "22 Ekim");
    /// assert_eq!(day.format(DateStyle::ShortDayMonth), "22 Eki");
    /// assert_eq!(day.format(DateStyle::MonthYear), "Ekim 2026");
    /// ```
    #[must_use]
    pub fn format(self, style: DateStyle) -> String {
        let (day, month, year) = (self.day, self.month, self.year);
        match style {
            DateStyle::Numeric => alloc::format!("{day:02}.{:02}.{year}", month.number()),
            DateStyle::Long => alloc::format!("{day} {} {year}", month.name()),
            DateStyle::LongWithWeekday => {
                alloc::format!("{day} {} {year} {}", month.name(), self.weekday().name())
            }
            DateStyle::LongCommaWeekday => {
                alloc::format!("{day} {} {year}, {}", month.name(), self.weekday().name())
            }
            DateStyle::DayMonth => alloc::format!("{day} {}", month.name()),
            DateStyle::ShortDayMonth => alloc::format!("{day} {}", month.short_name()),
            DateStyle::MonthYear => alloc::format!("{} {year}", month.name()),
        }
    }

    /// This date as a `jiff::civil::Date`, or `None` when the year is
    /// outside jiff's range.
    #[cfg(feature = "jiff")]
    #[cfg_attr(docsrs, doc(cfg(feature = "jiff")))]
    #[must_use]
    pub fn to_jiff(self) -> Option<jiff::civil::Date> {
        let year = i16::try_from(self.year).ok()?;
        let month = i8::try_from(self.month.number()).ok()?;
        let day = i8::try_from(self.day).ok()?;
        jiff::civil::Date::new(year, month, day).ok()
    }
}

#[cfg(feature = "jiff")]
#[cfg_attr(docsrs, doc(cfg(feature = "jiff")))]
impl From<jiff::civil::Date> for Date {
    /// ```
    /// use abece::date::Date;
    ///
    /// let day = Date::from(jiff::civil::date(2026, 7, 25));
    /// assert_eq!(day.to_string(), "25 Temmuz 2026");
    /// assert_eq!(day.to_jiff(), Some(jiff::civil::date(2026, 7, 25)));
    /// ```
    fn from(date: jiff::civil::Date) -> Self {
        // jiff's month and day are always in range, so neither fallback is reached.
        Self {
            year: i32::from(date.year()),
            month: Month::from_number(date.month().unsigned_abs()).unwrap_or(Month::January),
            day: date.day().unsigned_abs(),
        }
    }
}

#[cfg(feature = "jiff")]
#[cfg_attr(docsrs, doc(cfg(feature = "jiff")))]
impl From<jiff::civil::Weekday> for Weekday {
    /// ```
    /// use abece::date::Weekday;
    ///
    /// assert_eq!(Weekday::from(jiff::civil::Weekday::Saturday).name(), "Cumartesi");
    /// ```
    fn from(weekday: jiff::civil::Weekday) -> Self {
        match weekday {
            jiff::civil::Weekday::Monday => Weekday::Monday,
            jiff::civil::Weekday::Tuesday => Weekday::Tuesday,
            jiff::civil::Weekday::Wednesday => Weekday::Wednesday,
            jiff::civil::Weekday::Thursday => Weekday::Thursday,
            jiff::civil::Weekday::Friday => Weekday::Friday,
            jiff::civil::Weekday::Saturday => Weekday::Saturday,
            jiff::civil::Weekday::Sunday => Weekday::Sunday,
        }
    }
}

#[cfg(feature = "jiff")]
#[cfg_attr(docsrs, doc(cfg(feature = "jiff")))]
impl From<Weekday> for jiff::civil::Weekday {
    fn from(weekday: Weekday) -> Self {
        match weekday {
            Weekday::Monday => jiff::civil::Weekday::Monday,
            Weekday::Tuesday => jiff::civil::Weekday::Tuesday,
            Weekday::Wednesday => jiff::civil::Weekday::Wednesday,
            Weekday::Thursday => jiff::civil::Weekday::Thursday,
            Weekday::Friday => jiff::civil::Weekday::Friday,
            Weekday::Saturday => jiff::civil::Weekday::Saturday,
            Weekday::Sunday => jiff::civil::Weekday::Sunday,
        }
    }
}

impl fmt::Display for Date {
    /// The [`Long`](DateStyle::Long) style: "25 Temmuz 2026".
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {} {}", self.day, self.month.name(), self.year)
    }
}

/// `first` to `last` written out as a range: "20 Temmuz – 31 Ağustos 2026"
/// within one year, both dates in full across years, and a single date
/// when the two are the same day.
///
/// ```
/// use abece::date::{Date, format_range};
///
/// let range = |a: (i32, u8, u8), b: (i32, u8, u8)| {
///     format_range(Date::new(a.0, a.1, a.2).unwrap(), Date::new(b.0, b.1, b.2).unwrap())
/// };
/// assert_eq!(range((2026, 7, 20), (2026, 8, 31)), "20 Temmuz – 31 Ağustos 2026");
/// assert_eq!(range((2026, 12, 31), (2027, 1, 4)), "31 Aralık 2026 – 4 Ocak 2027");
/// assert_eq!(range((2026, 7, 20), (2026, 7, 20)), "20 Temmuz 2026");
/// ```
#[must_use]
pub fn format_range(first: Date, last: Date) -> String {
    if first == last {
        return first.format(DateStyle::Long);
    }
    if first.year == last.year {
        alloc::format!(
            "{} {} – {} {} {}",
            first.day,
            first.month.name(),
            last.day,
            last.month.name(),
            last.year
        )
    } else {
        alloc::format!(
            "{} – {}",
            first.format(DateStyle::Long),
            last.format(DateStyle::Long)
        )
    }
}

/// Days since 1970-01-01 of a Gregorian date (Howard Hinnant's algorithm).
#[expect(
    clippy::arithmetic_side_effects,
    reason = "an i32 year in i64 arithmetic cannot overflow"
)]
const fn days_from_civil(year: i32, month: u8, day: u8) -> i64 {
    let year = year as i64 - if month <= 2 { 1 } else { 0 };
    let era = year.div_euclid(400);
    let year_of_era = year.rem_euclid(400);
    let month = month as i64;
    let day_of_year =
        (153 * (if month > 2 { month - 3 } else { month + 9 }) + 2) / 5 + day as i64 - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::string::ToString;

    fn date(year: i32, month: u8, day: u8) -> Date {
        Date::new(year, month, day).unwrap()
    }

    #[test]
    fn weekdays_match_known_dates() {
        assert_eq!(date(2026, 7, 25).weekday(), Weekday::Saturday);
        assert_eq!(date(2026, 9, 8).weekday(), Weekday::Tuesday);
        assert_eq!(date(2027, 2, 1).weekday(), Weekday::Monday);
        assert_eq!(date(2026, 12, 31).weekday(), Weekday::Thursday);
        assert_eq!(date(1970, 1, 1).weekday(), Weekday::Thursday);
        assert_eq!(date(1969, 12, 31).weekday(), Weekday::Wednesday);
        assert_eq!(date(1900, 1, 1).weekday(), Weekday::Monday);
        assert_eq!(date(1600, 1, 1).weekday(), Weekday::Saturday);
        assert_eq!(date(-1, 12, 31).weekday(), Weekday::Friday);
        assert_eq!(days_from_civil(1970, 1, 1), 0);
        assert_eq!(days_from_civil(2000, 3, 1), 11_017);
    }

    #[test]
    fn every_style_and_the_range() {
        assert_eq!(
            date(2026, 7, 25).format(DateStyle::LongWithWeekday),
            "25 Temmuz 2026 Cumartesi"
        );
        assert_eq!(
            date(2026, 9, 8).format(DateStyle::LongWithWeekday),
            "8 Eylül 2026 Salı"
        );
        assert_eq!(
            date(2027, 2, 1).format(DateStyle::LongWithWeekday),
            "1 Şubat 2027 Pazartesi"
        );
        assert_eq!(
            date(2026, 12, 31).format(DateStyle::LongWithWeekday),
            "31 Aralık 2026 Perşembe"
        );
        assert_eq!(date(2026, 2, 1).format(DateStyle::Numeric), "01.02.2026");
        assert_eq!(
            date(2026, 10, 22).format(DateStyle::LongCommaWeekday),
            "22 Ekim 2026, Perşembe"
        );
        assert_eq!(
            date(2026, 10, 30).format(DateStyle::ShortDayMonth),
            "30 Eki"
        );
        assert_eq!(date(2026, 8, 3).format(DateStyle::ShortDayMonth), "3 Ağu");
        assert_eq!(date(2027, 2, 1).format(DateStyle::ShortDayMonth), "1 Şub");
        assert_eq!(date(2026, 10, 30).weekday().short_name(), "Cum");
        assert_eq!(date(2026, 11, 4).weekday().short_name(), "Çar");
        assert_eq!(date(2026, 7, 25).to_string(), "25 Temmuz 2026");
        assert_eq!(
            format_range(date(2026, 7, 20), date(2026, 8, 31)),
            "20 Temmuz – 31 Ağustos 2026"
        );
        assert_eq!(
            format_range(date(2026, 12, 31), date(2027, 1, 4)),
            "31 Aralık 2026 – 4 Ocak 2027"
        );
    }

    #[test]
    fn the_tables_are_complete() {
        for (index, month) in Month::ALL.iter().enumerate() {
            assert_eq!(usize::from(month.number()), index + 1);
            assert_eq!(Month::from_number(month.number()), Some(*month));
            assert_eq!(month.short_name().chars().count(), 3);
            assert!(month.name().starts_with(month.short_name()));
        }
        for (index, weekday) in Weekday::ALL.iter().enumerate() {
            assert_eq!(usize::from(weekday.number()), index + 1);
            assert_eq!(Weekday::from_number(weekday.number()), Some(*weekday));
            assert_eq!(weekday.short_name().chars().count(), 3);
        }
    }
}
