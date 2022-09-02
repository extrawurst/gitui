use once_cell::sync::Lazy;
use std::borrow::Cow;

static EMOJI_REPLACER: Lazy<gh_emoji::Replacer> =
	Lazy::new(gh_emoji::Replacer::new);

// Replace markdown emojis with Unicode equivalent
// :hammer: --> 🔨
#[inline]
pub fn emojifi_string(s: &mut String) {
	let resulting_cow = EMOJI_REPLACER.replace_all(s);
	if let Cow::Owned(altered_s) = resulting_cow {
		*s = altered_s;
	}
}
