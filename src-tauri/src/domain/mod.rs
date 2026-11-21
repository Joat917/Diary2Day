pub mod day_key;
pub mod name_key;
pub mod title;
pub mod words;

use chrono::{SecondsFormat, Utc};

pub fn utc_now_iso() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
}

pub fn local_offset_minutes() -> i32 {
    chrono::Local::now().offset().local_minus_utc() / 60
}
