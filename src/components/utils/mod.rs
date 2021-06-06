use chrono::{DateTime, Local, NaiveDateTime, Utc};
use unicode_width::UnicodeWidthStr;

pub mod filetree;
pub mod logitems;
pub mod scroll_vertical;
pub mod statustree;

/// macro to simplify running code that might return Err.
/// It will show a popup in that case
#[macro_export]
macro_rules! try_or_popup {
    ($self:ident, $msg:literal, $e:expr) => {
        if let Err(err) = $e {
            ::log::error!("{} {}", $msg, err);
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

#[inline]
pub fn string_width_align(s: &str, width: usize) -> String {
    static POSTFIX: &str = "..";

    let len = UnicodeWidthStr::width(s);
    let width_wo_postfix = width.saturating_sub(POSTFIX.len());

    if (len >= width_wo_postfix && len <= width)
        || (len <= width_wo_postfix)
    {
        format!("{:w$}", s, w = width)
    } else {
        let mut s = s.to_string();
        s.truncate(find_truncate_point(&s, width_wo_postfix));
        format!("{}{}", s, POSTFIX)
    }
}

#[inline]
fn find_truncate_point(s: &str, chars: usize) -> usize {
    s.chars().take(chars).map(char::len_utf8).sum()
}
