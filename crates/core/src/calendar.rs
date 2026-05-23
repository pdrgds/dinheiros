use chrono::{Datelike, Duration, NaiveDate, Weekday};
use std::collections::HashSet;

fn easter(year: i32) -> NaiveDate {
    let a = year % 19;
    let b = year / 100;
    let c = year % 100;
    let d = b / 4;
    let e = b % 4;
    let f = (b + 8) / 25;
    let g = (b - f + 1) / 3;
    let h = (19 * a + b - d - g + 15) % 30;
    let i = c / 4;
    let k = c % 4;
    let l = (32 + 2 * e + 2 * i - h - k) % 7;
    let m = (a + 11 * h + 22 * l) / 451;
    let month = (h + l - 7 * m + 114) / 31;
    let day = ((h + l - 7 * m + 114) % 31) + 1;
    NaiveDate::from_ymd_opt(year, month as u32, day as u32).unwrap()
}

fn nth_weekday(year: i32, month: u32, wd: Weekday, n: u32) -> NaiveDate {
    let first = NaiveDate::from_ymd_opt(year, month, 1).unwrap();
    let first_wd = first.weekday().num_days_from_monday() as i64;
    let target = wd.num_days_from_monday() as i64;
    let offset = (target - first_wd).rem_euclid(7) + 7 * (n as i64 - 1);
    first + Duration::days(offset)
}

fn last_weekday_of_month(year: i32, month: u32, wd: Weekday) -> NaiveDate {
    let last = if month == 12 {
        NaiveDate::from_ymd_opt(year + 1, 1, 1).unwrap() - Duration::days(1)
    } else {
        NaiveDate::from_ymd_opt(year, month + 1, 1).unwrap() - Duration::days(1)
    };
    let last_wd = last.weekday().num_days_from_monday() as i64;
    let target = wd.num_days_from_monday() as i64;
    let offset = (last_wd - target).rem_euclid(7);
    last - Duration::days(offset)
}

// NYSE moves fixed-date holidays to the nearest weekday when they fall on a weekend.
fn observe_us(d: NaiveDate) -> NaiveDate {
    match d.weekday() {
        Weekday::Sat => d - Duration::days(1),
        Weekday::Sun => d + Duration::days(1),
        _ => d,
    }
}

fn nyse_holidays(year: i32) -> HashSet<NaiveDate> {
    let e = easter(year);
    let mut s = HashSet::new();
    s.insert(observe_us(NaiveDate::from_ymd_opt(year, 1, 1).unwrap()));
    s.insert(nth_weekday(year, 1, Weekday::Mon, 3));
    s.insert(nth_weekday(year, 2, Weekday::Mon, 3));
    s.insert(e - Duration::days(2));
    s.insert(last_weekday_of_month(year, 5, Weekday::Mon));
    if year >= 2022 {
        s.insert(observe_us(NaiveDate::from_ymd_opt(year, 6, 19).unwrap()));
    }
    s.insert(observe_us(NaiveDate::from_ymd_opt(year, 7, 4).unwrap()));
    s.insert(nth_weekday(year, 9, Weekday::Mon, 1));
    s.insert(nth_weekday(year, 11, Weekday::Thu, 4));
    s.insert(observe_us(NaiveDate::from_ymd_opt(year, 12, 25).unwrap()));
    // Ad-hoc closures (national days of mourning, weather, etc.) — these aren't
    // derivable from the recurring rules, so each one has to be listed.
    for d in nyse_ad_hoc_closures(year) {
        s.insert(d);
    }
    s
}

fn nyse_ad_hoc_closures(year: i32) -> Vec<NaiveDate> {
    match year {
        // President Carter's state funeral — declared by NYSE Dec 2024.
        2025 => vec![NaiveDate::from_ymd_opt(2025, 1, 9).unwrap()],
        _ => Vec::new(),
    }
}

// B3 does NOT shift fixed-date holidays when they fall on weekends — they're just skipped.
fn b3_holidays(year: i32) -> HashSet<NaiveDate> {
    let e = easter(year);
    let mut s = HashSet::new();
    s.insert(NaiveDate::from_ymd_opt(year, 1, 1).unwrap());
    s.insert(e - Duration::days(48));
    s.insert(e - Duration::days(47));
    s.insert(e - Duration::days(2));
    s.insert(NaiveDate::from_ymd_opt(year, 4, 21).unwrap());
    s.insert(NaiveDate::from_ymd_opt(year, 5, 1).unwrap());
    s.insert(e + Duration::days(60));
    s.insert(NaiveDate::from_ymd_opt(year, 9, 7).unwrap());
    s.insert(NaiveDate::from_ymd_opt(year, 10, 12).unwrap());
    s.insert(NaiveDate::from_ymd_opt(year, 11, 2).unwrap());
    s.insert(NaiveDate::from_ymd_opt(year, 11, 15).unwrap());
    if year >= 2024 {
        s.insert(NaiveDate::from_ymd_opt(year, 11, 20).unwrap());
    }
    s.insert(NaiveDate::from_ymd_opt(year, 12, 25).unwrap());
    s
}

/// Count trading days inclusive of both endpoints for the given asset-type calendar.
///
/// - `crypto`: every calendar day (markets never close)
/// - `stock_br`: B3 calendar (Mon–Fri minus Brazilian national/financial holidays)
/// - `stock_intl`: NYSE calendar (Mon–Fri minus US market holidays, with weekend observance)
/// - anything else: Mon–Fri only, no holiday subtraction
pub fn trading_days(first: NaiveDate, last: NaiveDate, asset_type: &str) -> i64 {
    if last < first {
        return 0;
    }
    if asset_type == "crypto" {
        return (last - first).num_days() + 1;
    }

    let holidays: HashSet<NaiveDate> = match asset_type {
        "stock_br" => (first.year()..=last.year()).flat_map(b3_holidays).collect(),
        "stock_intl" => (first.year()..=last.year())
            .flat_map(nyse_holidays)
            .collect(),
        _ => HashSet::new(),
    };

    let mut count = 0i64;
    let mut d = first;
    while d <= last {
        let wd = d.weekday();
        if wd != Weekday::Sat && wd != Weekday::Sun && !holidays.contains(&d) {
            count += 1;
        }
        d = d.succ_opt().unwrap();
    }
    count
}

/// Whether the given date is a trading day for the asset type.
pub fn is_trading_day(d: NaiveDate, asset_type: &str) -> bool {
    if asset_type == "crypto" {
        return true;
    }
    let wd = d.weekday();
    if wd == Weekday::Sat || wd == Weekday::Sun {
        return false;
    }
    match asset_type {
        "stock_br" => !b3_holidays(d.year()).contains(&d),
        "stock_intl" => !nyse_holidays(d.year()).contains(&d),
        _ => true,
    }
}

/// Sorted list of trading days in `[first, last]` (inclusive) that are NOT in `present`.
pub fn missing_trading_days(
    first: NaiveDate,
    last: NaiveDate,
    asset_type: &str,
    present: &HashSet<NaiveDate>,
) -> Vec<NaiveDate> {
    if last < first {
        return Vec::new();
    }
    let mut missing = Vec::new();
    let mut d = first;
    while d <= last {
        if is_trading_day(d, asset_type) && !present.contains(&d) {
            missing.push(d);
        }
        d = d.succ_opt().unwrap();
    }
    missing
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn easter_known_dates() {
        assert_eq!(easter(2024), NaiveDate::from_ymd_opt(2024, 3, 31).unwrap());
        assert_eq!(easter(2023), NaiveDate::from_ymd_opt(2023, 4, 9).unwrap());
        assert_eq!(easter(2025), NaiveDate::from_ymd_opt(2025, 4, 20).unwrap());
    }

    #[test]
    fn nyse_2024_has_252_trading_days() {
        let days = trading_days(
            NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            NaiveDate::from_ymd_opt(2024, 12, 31).unwrap(),
            "stock_intl",
        );
        assert_eq!(days, 252);
    }

    #[test]
    fn nyse_2023_has_250_trading_days() {
        let days = trading_days(
            NaiveDate::from_ymd_opt(2023, 1, 1).unwrap(),
            NaiveDate::from_ymd_opt(2023, 12, 31).unwrap(),
            "stock_intl",
        );
        assert_eq!(days, 250);
    }

    #[test]
    fn b3_2024_trading_days_in_range() {
        let days = trading_days(
            NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            NaiveDate::from_ymd_opt(2024, 12, 31).unwrap(),
            "stock_br",
        );
        // 2024 = 262 weekdays. B3 closes ~9 on weekdays (others fall on Sat/Sun).
        // Half-day sessions (Dec 24/31) count as trading days because Yahoo stores
        // a close price for them.
        assert!((248..=255).contains(&days), "expected 248–255, got {days}");
    }

    #[test]
    fn crypto_counts_every_day() {
        let days = trading_days(
            NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            NaiveDate::from_ymd_opt(2024, 12, 31).unwrap(),
            "crypto",
        );
        assert_eq!(days, 366); // 2024 is a leap year
    }

    #[test]
    fn empty_range() {
        let d = NaiveDate::from_ymd_opt(2024, 5, 1).unwrap();
        assert_eq!(trading_days(d, d - Duration::days(1), "stock_intl"), 0);
    }

    #[test]
    fn nyse_carter_day_of_mourning_2025() {
        // NYSE was closed Thu 2025-01-09 for President Carter's state funeral —
        // an ad-hoc closure not derivable from the recurring holiday rules.
        let day = NaiveDate::from_ymd_opt(2025, 1, 9).unwrap();
        assert!(!is_trading_day(day, "stock_intl"));
        assert!(nyse_holidays(2025).contains(&day));
    }

    #[test]
    fn us_independence_day_weekend_observance() {
        // Jul 4 2026 is a Saturday; NYSE observes Friday Jul 3.
        let hs = nyse_holidays(2026);
        assert!(hs.contains(&NaiveDate::from_ymd_opt(2026, 7, 3).unwrap()));
        assert!(!hs.contains(&NaiveDate::from_ymd_opt(2026, 7, 4).unwrap()));
    }

    #[test]
    fn is_trading_day_crypto_always_true() {
        let sat = NaiveDate::from_ymd_opt(2026, 4, 18).unwrap();
        let sun = NaiveDate::from_ymd_opt(2026, 4, 19).unwrap();
        assert!(is_trading_day(sat, "crypto"));
        assert!(is_trading_day(sun, "crypto"));
    }

    #[test]
    fn is_trading_day_nyse_skips_weekends_and_holidays() {
        let fri = NaiveDate::from_ymd_opt(2026, 4, 17).unwrap();
        let sat = NaiveDate::from_ymd_opt(2026, 4, 18).unwrap();
        let sun = NaiveDate::from_ymd_opt(2026, 4, 19).unwrap();
        let good_friday = NaiveDate::from_ymd_opt(2026, 4, 3).unwrap();
        assert!(is_trading_day(fri, "stock_intl"));
        assert!(!is_trading_day(sat, "stock_intl"));
        assert!(!is_trading_day(sun, "stock_intl"));
        assert!(!is_trading_day(good_friday, "stock_intl"));
    }

    #[test]
    fn is_trading_day_b3_skips_brazilian_holidays() {
        let independencia = NaiveDate::from_ymd_opt(2026, 9, 7).unwrap();
        let normal_mon = NaiveDate::from_ymd_opt(2026, 9, 14).unwrap();
        assert!(!is_trading_day(independencia, "stock_br"));
        assert!(is_trading_day(normal_mon, "stock_br"));
    }

    #[test]
    fn missing_trading_days_finds_middle_gaps() {
        let first = NaiveDate::from_ymd_opt(2026, 4, 1).unwrap();
        let last = NaiveDate::from_ymd_opt(2026, 4, 17).unwrap();
        let mut present = HashSet::new();
        for d in [(2026, 4, 1), (2026, 4, 2), (2026, 4, 6), (2026, 4, 15)] {
            present.insert(NaiveDate::from_ymd_opt(d.0, d.1, d.2).unwrap());
        }
        let missing = missing_trading_days(first, last, "stock_intl", &present);
        assert_eq!(
            missing,
            vec![
                NaiveDate::from_ymd_opt(2026, 4, 7).unwrap(),
                NaiveDate::from_ymd_opt(2026, 4, 8).unwrap(),
                NaiveDate::from_ymd_opt(2026, 4, 9).unwrap(),
                NaiveDate::from_ymd_opt(2026, 4, 10).unwrap(),
                NaiveDate::from_ymd_opt(2026, 4, 13).unwrap(),
                NaiveDate::from_ymd_opt(2026, 4, 14).unwrap(),
                NaiveDate::from_ymd_opt(2026, 4, 16).unwrap(),
                NaiveDate::from_ymd_opt(2026, 4, 17).unwrap(),
            ]
        );
    }

    #[test]
    fn missing_trading_days_ignores_weekend_pollution() {
        // Even if `present` contains weekend rows (live-price bug), they shouldn't
        // count as "expected" days, so missing remains the same.
        let first = NaiveDate::from_ymd_opt(2026, 4, 13).unwrap();
        let last = NaiveDate::from_ymd_opt(2026, 4, 19).unwrap();
        let mut present = HashSet::new();
        present.insert(NaiveDate::from_ymd_opt(2026, 4, 19).unwrap()); // Sunday
        let missing = missing_trading_days(first, last, "stock_intl", &present);
        assert_eq!(missing.len(), 5); // Mon Tue Wed Thu Fri all missing
    }

    #[test]
    fn missing_trading_days_empty_when_complete() {
        let first = NaiveDate::from_ymd_opt(2026, 4, 13).unwrap();
        let last = NaiveDate::from_ymd_opt(2026, 4, 17).unwrap();
        let mut present = HashSet::new();
        for day in 13..=17 {
            present.insert(NaiveDate::from_ymd_opt(2026, 4, day).unwrap());
        }
        let missing = missing_trading_days(first, last, "stock_intl", &present);
        assert!(missing.is_empty());
    }

    #[test]
    fn missing_trading_days_crypto_includes_every_day() {
        let first = NaiveDate::from_ymd_opt(2026, 4, 13).unwrap();
        let last = NaiveDate::from_ymd_opt(2026, 4, 19).unwrap();
        let present = HashSet::new();
        let missing = missing_trading_days(first, last, "crypto", &present);
        assert_eq!(missing.len(), 7);
    }
}
