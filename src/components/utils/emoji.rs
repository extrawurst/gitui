use lazy_static::lazy_static;
use std::borrow::Cow;

lazy_static! {
	static ref EMOJI_REPLACER: gh_emoji::Replacer =
		gh_emoji::Replacer::new();
}

// Replace markdown emojis with Unicode equivalent
// :hammer: --> 🔨
#[inline]
pub fn emojifi_string(s: &mut String) {
	let resulting_cow = EMOJI_REPLACER.replace_all(s);
	if let Cow::Owned(altered_s) = resulting_cow {
		*s = altered_s;
	}
}
