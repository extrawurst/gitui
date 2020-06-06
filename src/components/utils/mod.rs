use chrono::{DateTime, Local, NaiveDateTime, Utc};

pub mod filetree;
pub mod logitems;
pub mod statustree;

/// helper func to convert unix time since epoch to formated time string in local timezone
pub fn time_to_string(secs: i64, short: bool) -> String {
    let time = DateTime::<Local>::from(DateTime::<Utc>::from_utc(
        NaiveDateTime::from_timestamp(secs, 0),
        Utc,
    ));
    time.format(if short {
        "%Y-%m-%d"
    } else {
        "%Y-%m-%d %H:%M:%S"
    })
    .to_string()
}
