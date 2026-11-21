use chrono::{DateTime, FixedOffset};

use crate::error::AppError;

pub fn compute_day_key(recorded_at: &str, offset_minutes: i32) -> Result<String, AppError> {
    let utc: DateTime<FixedOffset> = DateTime::parse_from_rfc3339(recorded_at)?;
    let offset = FixedOffset::east_opt(offset_minutes * 60)
        .ok_or_else(|| AppError::new("bad_offset", "时区偏移超出范围"))?;
    Ok(utc.with_timezone(&offset).format("%Y-%m-%d").to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn late_night_stays_on_recorded_day() {
        assert_eq!(
            compute_day_key("2026-11-21T16:30:00Z", 480).unwrap(),
            "2026-11-22"
        );
        assert_eq!(
            compute_day_key("2026-11-21T16:30:00Z", 0).unwrap(),
            "2026-11-21"
        );
        assert_eq!(
            compute_day_key("2026-11-21T23:30:00Z", -300).unwrap(),
            "2026-11-21"
        );
    }

    #[test]
    fn half_hour_offset_boundary() {
        assert_eq!(
            compute_day_key("2026-11-21T18:29:59Z", 330).unwrap(),
            "2026-11-21"
        );
        assert_eq!(
            compute_day_key("2026-11-21T18:30:00Z", 330).unwrap(),
            "2026-11-22"
        );
    }
}
