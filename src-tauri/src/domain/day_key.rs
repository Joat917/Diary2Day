use chrono::{DateTime, Duration, FixedOffset};

use crate::error::AppError;

pub fn compute_day_key(
    recorded_at: &str,
    offset_minutes: i32,
    boundary_minutes: u32,
) -> Result<String, AppError> {
    let utc: DateTime<FixedOffset> = DateTime::parse_from_rfc3339(recorded_at)?;
    let offset = FixedOffset::east_opt(offset_minutes * 60)
        .ok_or_else(|| AppError::new("bad_offset", "时区偏移超出范围"))?;
    let local = utc.with_timezone(&offset);
    let boundary = Duration::minutes(i64::from(boundary_minutes.min(479)));
    Ok((local - boundary).format("%Y-%m-%d").to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn late_night_stays_on_recorded_day_without_boundary() {
        assert_eq!(
            compute_day_key("2026-11-21T16:30:00Z", 480, 0).unwrap(),
            "2026-11-22"
        );
        assert_eq!(
            compute_day_key("2026-11-21T16:30:00Z", 0, 0).unwrap(),
            "2026-11-21"
        );
        assert_eq!(
            compute_day_key("2026-11-21T23:30:00Z", -300, 0).unwrap(),
            "2026-11-21"
        );
    }

    #[test]
    fn half_hour_offset_boundary() {
        assert_eq!(
            compute_day_key("2026-11-21T18:29:59Z", 330, 0).unwrap(),
            "2026-11-21"
        );
        assert_eq!(
            compute_day_key("2026-11-21T18:30:00Z", 330, 0).unwrap(),
            "2026-11-22"
        );
    }

    #[test]
    fn boundary_rolls_early_morning_to_previous_day() {
        assert_eq!(
            compute_day_key("2026-11-21T17:00:00Z", 480, 240).unwrap(),
            "2026-11-21"
        );
        assert_eq!(
            compute_day_key("2026-11-21T19:59:59Z", 480, 240).unwrap(),
            "2026-11-21"
        );
        assert_eq!(
            compute_day_key("2026-11-21T20:00:00Z", 480, 240).unwrap(),
            "2026-11-22"
        );
        assert_eq!(
            compute_day_key("2026-11-21T10:00:00Z", 480, 240).unwrap(),
            "2026-11-21"
        );
    }
}