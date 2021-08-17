use asyncgit::asyncjob::AsyncJob;
use lazy_static::lazy_static;
use scopetime::scope_time;
use std::{
	ffi::OsStr,
	ops::Range,
	path::{Path, PathBuf},
	sync::{Arc, Mutex},
};
use syntect::{
	highlighting::{
		FontStyle, HighlightState, Highlighter,
		RangedHighlightIterator, Style, ThemeSet,
	},
	parsing::{ParseState, ScopeStack, SyntaxSet},
};
use tui::text::{Span, Spans};

struct SyntaxLine {
	items: Vec<(Style, usize, Range<usize>)>,
}

pub struct SyntaxText {
	text: String,
	lines: Vec<SyntaxLine>,
	path: PathBuf,
}

lazy_static! {
	static ref SYNTAX_SET: SyntaxSet =
		SyntaxSet::load_defaults_nonewlines();
	static ref THEME_SET: ThemeSet = ThemeSet::load_defaults();
}

impl SyntaxText {
	pub fn new(text: String, file_path: &Path) -> Self {
		scope_time!("syntax_highlighting");
		log::debug!("syntax: {:?}", file_path);

		let mut state = {
			let syntax = file_path
				.extension()
				.and_then(OsStr::to_str)
				.map_or_else(
					|| {
						SYNTAX_SET.find_syntax_by_path(
							file_path.to_str().unwrap_or_default(),
						)
					},
					|ext| SYNTAX_SET.find_syntax_by_extension(ext),
				);

			ParseState::new(syntax.unwrap_or_else(|| {
				SYNTAX_SET.find_syntax_plain_text()
			}))
		};

		let highlighter = Highlighter::new(
			&THEME_SET.themes["base16-eighties.dark"],
		);

		let mut syntax_lines: Vec<SyntaxLine> = Vec::new();

		let mut highlight_state =
			HighlightState::new(&highlighter, ScopeStack::new());

		for (number, line) in text.lines().enumerate() {
			let ops = state.parse_line(line, &SYNTAX_SET);
			let iter = RangedHighlightIterator::new(
				&mut highlight_state,
				&ops[..],
				line,
				&highlighter,
			);

			syntax_lines.push(SyntaxLine {
				items: iter
					.map(|(style, _, range)| (style, number, range))
					.collect(),
			});
		}

		Self {
			text,
			lines: syntax_lines,
			path: file_path.into(),
		}
	}

	///
	pub fn path(&self) -> &Path {
		&self.path
	}
}

impl<'a> From<&'a SyntaxText> for tui::text::Text<'a> {
	fn from(v: &'a SyntaxText) -> Self {
		let mut result_lines: Vec<Spans> =
			Vec::with_capacity(v.lines.len());

		for (syntax_line, line_content) in
			v.lines.iter().zip(v.text.lines())
		{
			let mut line_span =
				Spans(Vec::with_capacity(syntax_line.items.len()));

			for (style, _, range) in &syntax_line.items {
				let item_content = &line_content[range.clone()];
				let item_style = syntact_style_to_tui(style);

				line_span
					.0
					.push(Span::styled(item_content, item_style));
			}

			result_lines.push(line_span);
		}

		result_lines.into()
	}
}

fn syntact_style_to_tui(style: &Style) -> tui::style::Style {
	let mut res =
		tui::style::Style::default().fg(tui::style::Color::Rgb(
			style.foreground.r,
			style.foreground.g,
			style.foreground.b,
		));

	if style.font_style.contains(FontStyle::BOLD) {
		res = res.add_modifier(tui::style::Modifier::BOLD);
	}
	if style.font_style.contains(FontStyle::ITALIC) {
		res = res.add_modifier(tui::style::Modifier::ITALIC);
	}
	if style.font_style.contains(FontStyle::UNDERLINE) {
		res = res.add_modifier(tui::style::Modifier::UNDERLINED);
	}

	res
}

enum JobState {
	Request((String, String)),
	Response(SyntaxText),
}

#[derive(Clone, Default)]
pub struct AsyncSyntaxJob {
	state: Arc<Mutex<Option<JobState>>>,
}

impl AsyncSyntaxJob {
	pub fn new(content: String, path: String) -> Self {
		Self {
			state: Arc::new(Mutex::new(Some(JobState::Request((
				content, path,
			))))),
		}
	}

	pub fn result(&self) -> Option<SyntaxText> {
		if let Ok(mut state) = self.state.lock() {
			if let Some(state) = state.take() {
				return match state {
					JobState::Request(_) => None,
					JobState::Response(text) => Some(text),
				};
			}
		}

		None
	}
}

impl AsyncJob for AsyncSyntaxJob {
	fn run(&mut self) {
		if let Ok(mut state) = self.state.lock() {
			*state = state.take().map(|state| match state {
				JobState::Request((content, path)) => {
					let syntax =
						SyntaxText::new(content, Path::new(&path));
					JobState::Response(syntax)
				}
				JobState::Response(res) => JobState::Response(res),
			});
		}
	}
}
