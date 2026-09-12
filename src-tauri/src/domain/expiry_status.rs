//! Expiry status classification for dashboard grouping.
//!
//! Pure domain logic — no I/O.

use chrono::NaiveDate;

/// Urgency level ordered from most to least urgent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Urgency {
    Expired = 0,
    Today = 1,
    AlertWindow = 2,
    Next30Days = 3,
    Future = 4,
}

/// Classifies a lot's urgency based on today and expiry date.
pub fn classify_urgency(today: NaiveDate, expiry_date: NaiveDate) -> Urgency {
    if today > expiry_date {
        Urgency::Expired
    } else if today == expiry_date {
        Urgency::Today
    } else {
        let diff = (expiry_date - today).num_days();
        if diff <= 30 {
            Urgency::Next30Days
        } else {
            Urgency::Future
        }
    }
}

/// Returns `true` when `alert_days_before > 0` and today is inside the alert window.
pub fn is_due_for_notification(
    today: NaiveDate,
    expiry_date: NaiveDate,
    alert_days_before: i32,
) -> bool {
    if alert_days_before <= 0 {
        return false;
    }
    let diff = (expiry_date - today).num_days();
    diff >= 0 && diff <= alert_days_before as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }

    #[test]
    fn classify_urgency_ordering() {
        assert!(Urgency::Expired < Urgency::Today);
        assert!(Urgency::Today < Urgency::AlertWindow);
        assert!(Urgency::AlertWindow < Urgency::Next30Days);
    }

    #[test]
    fn classify_expired() {
        let today = d(2025, 7, 1);
        let expiry = d(2025, 6, 30);
        assert_eq!(classify_urgency(today, expiry), Urgency::Expired);
    }

    #[test]
    fn classify_today() {
        let today = d(2025, 6, 30);
        assert_eq!(classify_urgency(today, today), Urgency::Today);
    }

    #[test]
    fn classify_next_30_days() {
        let today = d(2025, 6, 15);
        // 10 days out
        assert_eq!(classify_urgency(today, d(2025, 6, 25)), Urgency::Next30Days);
        // exactly 30
        assert_eq!(classify_urgency(today, d(2025, 7, 15)), Urgency::Next30Days);
    }

    #[test]
    fn classify_future() {
        let today = d(2025, 6, 1);
        assert_eq!(classify_urgency(today, d(2025, 8, 1)), Urgency::Future);
    }

    #[test]
    fn is_due_for_notification_works() {
        let expiry = d(2025, 6, 30);
        // inside window
        assert!(is_due_for_notification(d(2025, 6, 15), expiry, 30));
        // on expiry edge
        assert!(is_due_for_notification(d(2025, 6, 30), expiry, 30));
        // outside window (before)
        assert!(!is_due_for_notification(d(2025, 5, 30), expiry, 30));
        // zero alert days
        assert!(!is_due_for_notification(d(2025, 6, 30), expiry, 0));
    }
}
