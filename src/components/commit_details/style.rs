use crate::{keys::SharedKeyConfig, strings, ui::style::SharedTheme};
use std::borrow::Cow;
use tui::text::Span;

pub enum Detail {
	Author,
	Date,
	Commiter,
	Sha,
}

pub fn style_detail<'a>(
	theme: &'a SharedTheme,
	keys: &'a SharedKeyConfig,
	field: &Detail,
) -> Span<'a> {
	match field {
		Detail::Author => Span::styled(
			Cow::from(strings::commit::details_author(keys)),
			theme.text(false, false),
		),
		Detail::Date => Span::styled(
			Cow::from(strings::commit::details_date(keys)),
			theme.text(false, false),
		),
		Detail::Commiter => Span::styled(
			Cow::from(strings::commit::details_committer(keys)),
			theme.text(false, false),
		),
		Detail::Sha => Span::styled(
			Cow::from(strings::commit::details_tags(keys)),
			theme.text(false, false),
		),
	}
}
