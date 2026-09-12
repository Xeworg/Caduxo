//! Alert window calculation logic.
//!
//! Pure functions — no I/O, no external dependencies.

use chrono::{NaiveDate, Utc};

/// Computes the start of the alert window for a lot.
///
/// `alert_start = expiry_date - alert_days_before`
///
/// Returns `None` if the result would be a date before year 1 (underflow).
pub fn alert_start_date(expiry_date: NaiveDate, alert_days_before: i32) -> Option<NaiveDate> {
    expiry_date.checked_sub_signed(chrono::Duration::days(alert_days_before as i64))
}

/// Returns `true` if `today` falls within the inclusive alert window
/// `[alert_start, expiry_date]`.
pub fn is_in_alert_window(
    today: NaiveDate,
    expiry_date: NaiveDate,
    alert_days_before: i32,
) -> bool {
    let Some(alert_start) = alert_start_date(expiry_date, alert_days_before) else {
        return false;
    };
    today >= alert_start && today <= expiry_date
}

/// Returns `true` if `today` is strictly after `expiry_date`.
pub fn is_expired(today: NaiveDate, expiry_date: NaiveDate) -> bool {
    today > expiry_date
}

/// Returns the number of days remaining until expiry (can be negative).
pub fn days_until_expiry(today: NaiveDate, expiry_date: NaiveDate) -> i64 {
    (expiry_date - today).num_days()
}

/// Returns today's date in UTC.
pub fn today_utc() -> NaiveDate {
    Utc::now().date_naive()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    #[test]
    fn alert_start_computes_correctly() {
        let expiry = d(2025, 6, 30);
        assert_eq!(alert_start_date(expiry, 30), Some(d(2025, 5, 31)));
        assert_eq!(alert_start_date(expiry, 0), Some(d(2025, 6, 30)));
        assert_eq!(alert_start_date(expiry, 90), Some(d(2025, 4, 1)));
    }

    #[test]
    fn is_in_alert_window_works() {
        let expiry = d(2025, 6, 30);
        let today = d(2025, 6, 15);
        let alert_days = 30;

        // before alert start
        assert!(!is_in_alert_window(d(2025, 5, 30), expiry, alert_days));
        // on alert start
        assert!(is_in_alert_window(d(2025, 5, 31), expiry, alert_days));
        // inside window
        assert!(is_in_alert_window(d(2025, 6, 15), expiry, alert_days));
        // on expiry
        assert!(is_in_alert_window(d(2025, 6, 30), expiry, alert_days));
        // after expiry
        assert!(!is_in_alert_window(d(2025, 7, 1), expiry, alert_days));
    }

    #[test]
    fn is_expired_works() {
        let expiry = d(2025, 6, 30);
        assert!(!is_expired(d(2025, 6, 30), expiry));
        assert!(is_expired(d(2025, 7, 1), expiry));
        assert!(!is_expired(d(2025, 6, 29), expiry));
    }

    #[test]
    fn days_until_expiry_works() {
        let expiry = d(2025, 6, 30);
        let today = d(2025, 6, 20);
        assert_eq!(days_until_expiry(today, expiry), 10);
        assert_eq!(days_until_expiry(expiry, expiry), 0);
        assert_eq!(days_until_expiry(d(2025, 7, 10), expiry), -10);
    }
}
