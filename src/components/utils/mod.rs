use chrono::{DateTime, Local, NaiveDateTime, Utc};

pub mod async_commit_filter;
pub mod filetree;
pub mod logitems;
pub mod statustree;

/// macro to simplify running code that might return Err.
/// It will show a popup in that case
#[macro_export]
macro_rules! try_or_popup {
    ($self:ident, $msg:literal, $e:expr) => {
        if let Err(err) = $e {
            $self.queue.borrow_mut().push_back(
                InternalEvent::ShowErrorMsg(format!(
                    "{}\n{}",
                    $msg, err
                )),
            );
        }
    };
}

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
