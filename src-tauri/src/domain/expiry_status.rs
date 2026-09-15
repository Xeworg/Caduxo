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

/// Classifies a lot's urgency using the lot's specific `alert_days_before` value.
///
/// Urgency order (most → least urgent):
///   Expired → Today → AlertWindow → Next30Days → Future
///
/// `AlertWindow` is used when the lot is within its alert window but not yet
/// at expiry. When `alert_days_before > 30`, the alert window overlaps with the
/// "next 30 days" bucket, so those lots are classified as `Next30Days`.
pub fn classify_urgency_with_alert(
    today: NaiveDate,
    expiry_date: NaiveDate,
    alert_days_before: i32,
) -> Urgency {
    if today > expiry_date {
        Urgency::Expired
    } else if today == expiry_date {
        Urgency::Today
    } else {
        let diff = (expiry_date - today).num_days();
        // "Alert window": lot is within its alert window (diff <= alert_days_before)
        // AND the alert threshold is strictly less than 30 days.
        if alert_days_before > 0
            && diff <= 30
            && diff <= alert_days_before as i64
            && alert_days_before < 30
        {
            Urgency::AlertWindow
        } else if diff <= 30 {
            Urgency::Next30Days
        } else {
            Urgency::Future
        }
    }
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
    fn classify_urgency_with_alert_alert_window() {
        let today = d(2025, 6, 15);
        // expiry in 10 days, alert_days=14 → within alert window → AlertWindow
        assert_eq!(
            classify_urgency_with_alert(today, d(2025, 6, 25), 14),
            Urgency::AlertWindow
        );
        // alert_days=30, expiry in 25 days → within 30 days but NOT AlertWindow
        // (alert_days < 30 is false) → Next30Days
        assert_eq!(
            classify_urgency_with_alert(today, d(2025, 7, 10), 30),
            Urgency::Next30Days
        );
        // alert_days=7, expiry in 10 days → NOT in alert window (10 > 7) → Next30Days
        assert_eq!(
            classify_urgency_with_alert(today, d(2025, 6, 25), 7),
            Urgency::Next30Days
        );
        // zero alert days → never AlertWindow → Next30Days
        assert_eq!(
            classify_urgency_with_alert(today, d(2025, 6, 25), 0),
            Urgency::Next30Days
        );
    }

    #[test]
    fn classify_urgency_with_alert_next_30_days() {
        let today = d(2025, 6, 15);
        // alert_days=30, expiry in 25 days → NOT AlertWindow (alert_days < 30 is false) → Next30Days
        assert_eq!(
            classify_urgency_with_alert(today, d(2025, 7, 10), 30),
            Urgency::Next30Days
        );
        // alert_days=60 (more than 30), expiry in 25 days → Next30Days
        assert_eq!(
            classify_urgency_with_alert(today, d(2025, 7, 10), 60),
            Urgency::Next30Days
        );
    }
}
