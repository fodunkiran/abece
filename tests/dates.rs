//! The names of the months and the days, every date style, and, with the
//! `jiff` feature, the conversions.
// Test code: the no-panic policy is for the library.
#![allow(
    clippy::arithmetic_side_effects,
    clippy::indexing_slicing,
    clippy::string_slice,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic
)]

use abece::date::{Date, DateStyle, Month, Weekday, format_range};

fn date(year: i32, month: u8, day: u8) -> Date {
    Date::new(year, month, day).unwrap()
}

#[test]
fn long_dates_with_the_weekday_and_ranges() {
    let format_date = |d: Date| d.format(DateStyle::LongWithWeekday);
    assert_eq!(format_date(date(2026, 7, 25)), "25 Temmuz 2026 Cumartesi");
    assert_eq!(format_date(date(2026, 9, 8)), "8 Eylül 2026 Salı");
    assert_eq!(format_date(date(2027, 2, 1)), "1 Şubat 2027 Pazartesi");
    assert_eq!(format_date(date(2026, 12, 31)), "31 Aralık 2026 Perşembe");
    assert_eq!(
        format_range(date(2026, 7, 20), date(2026, 8, 31)),
        "20 Temmuz – 31 Ağustos 2026"
    );
    assert_eq!(
        format_range(date(2026, 12, 31), date(2027, 1, 4)),
        "31 Aralık 2026 – 4 Ocak 2027"
    );
    assert_eq!(
        format_range(date(2026, 7, 20), date(2026, 7, 20)),
        "20 Temmuz 2026"
    );
}

#[test]
fn numeric_and_comma_styles() {
    assert_eq!(date(2026, 2, 1).format(DateStyle::Numeric), "01.02.2026");
    assert_eq!(
        date(2026, 10, 22).format(DateStyle::LongCommaWeekday),
        "22 Ekim 2026, Perşembe"
    );
    assert_eq!(
        date(2027, 2, 1).format(DateStyle::LongCommaWeekday),
        "1 Şubat 2027, Pazartesi"
    );
    assert_eq!(date(2026, 10, 22).format(DateStyle::MonthYear), "Ekim 2026");
    assert_eq!(date(2026, 10, 22).format(DateStyle::DayMonth), "22 Ekim");
}

#[test]
fn short_styles() {
    assert_eq!(
        date(2026, 10, 30).format(DateStyle::ShortDayMonth),
        "30 Eki"
    );
    assert_eq!(date(2026, 8, 3).format(DateStyle::ShortDayMonth), "3 Ağu");
    assert_eq!(date(2027, 2, 1).format(DateStyle::ShortDayMonth), "1 Şub");
    assert_eq!(date(2026, 10, 30).weekday().short_name(), "Cum");
    assert_eq!(date(2026, 11, 4).weekday().short_name(), "Çar");
}

#[test]
fn the_month_and_weekday_tables() {
    let months = [
        "Ocak", "Şubat", "Mart", "Nisan", "Mayıs", "Haziran", "Temmuz", "Ağustos", "Eylül", "Ekim",
        "Kasım", "Aralık",
    ];
    for (month, name) in Month::ALL.iter().zip(months) {
        assert_eq!(month.name(), name);
        // The short name is the first three letters of the name.
        assert_eq!(month.short_name(), name.chars().take(3).collect::<String>());
    }
    let weekdays = [
        ("Pazartesi", "Pzt"),
        ("Salı", "Sal"),
        ("Çarşamba", "Çar"),
        ("Perşembe", "Per"),
        ("Cuma", "Cum"),
        ("Cumartesi", "Cmt"),
        ("Pazar", "Paz"),
    ];
    for (weekday, (name, short)) in Weekday::ALL.iter().zip(weekdays) {
        assert_eq!(weekday.name(), name);
        assert_eq!(weekday.short_name(), short);
    }
}

#[cfg(feature = "jiff")]
#[test]
fn jiff_dates_convert_both_ways() {
    use jiff::civil;

    let day = civil::date(2026, 7, 25);
    let converted = Date::from(day);
    assert_eq!(
        converted.format(DateStyle::LongWithWeekday),
        "25 Temmuz 2026 Cumartesi"
    );
    assert_eq!(converted.to_jiff(), Some(day));
    assert_eq!(Weekday::from(day.weekday()), converted.weekday());
    assert_eq!(civil::Weekday::from(converted.weekday()), day.weekday());
    // Every day of a few decades agrees with jiff on the weekday.
    let mut day = civil::date(1999, 1, 1);
    while day.year() < 2031 {
        let ours = Date::new(
            i32::from(day.year()),
            day.month().unsigned_abs(),
            day.day().unsigned_abs(),
        )
        .unwrap();
        assert_eq!(Weekday::from(day.weekday()), ours.weekday(), "{day}");
        day = day.tomorrow().unwrap();
    }
    assert_eq!(Date::new(100_000, 1, 1).unwrap().to_jiff(), None);
}
