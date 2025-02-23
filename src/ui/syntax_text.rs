use asyncgit::{
	asyncjob::{AsyncJob, RunParams},
	ProgressPercent,
};
use once_cell::sync::Lazy;
use ratatui::text::{Line, Span};
use scopetime::scope_time;
use std::{
	ops::Range,
	path::{Path, PathBuf},
	sync::{Arc, Mutex},
	time::{Duration, Instant},
};
use syntect::{
	highlighting::{
		FontStyle, HighlightState, Highlighter,
		RangedHighlightIterator, Style, ThemeSet,
	},
	parsing::{ParseState, ScopeStack, SyntaxSet},
};

use crate::{AsyncAppNotification, SyntaxHighlightProgress};

struct SyntaxLine {
	items: Vec<(Style, usize, Range<usize>)>,
}

pub struct SyntaxText {
	text: String,
	lines: Vec<SyntaxLine>,
	path: PathBuf,
}

static SYNTAX_SET: Lazy<SyntaxSet> =
	Lazy::new(two_face::syntax::extra_no_newlines);
static THEME_SET: Lazy<ThemeSet> = Lazy::new(ThemeSet::load_defaults);

pub struct AsyncProgressBuffer {
	current: usize,
	total: usize,
	last_send: Option<Instant>,
	min_interval: Duration,
}

impl AsyncProgressBuffer {
	pub const fn new(total: usize, min_interval: Duration) -> Self {
		Self {
			current: 0,
			total,
			last_send: None,
			min_interval,
		}
	}

	pub fn send_progress(&mut self) -> ProgressPercent {
		self.last_send = Some(Instant::now());
		ProgressPercent::new(self.current, self.total)
	}

	pub fn update(&mut self, current: usize) -> bool {
		self.current = current;
		self.last_send.map_or(true, |last_send| {
			last_send.elapsed() > self.min_interval
		})
	}
}

impl SyntaxText {
	pub fn new(
		text: String,
		file_path: &Path,
		params: &RunParams<AsyncAppNotification, ProgressPercent>,
		syntax: &str,
	) -> asyncgit::Result<Self> {
		scope_time!("syntax_highlighting");
		let mut state = {
			scope_time!("syntax_highlighting.0");
			let plain_text = || SYNTAX_SET.find_syntax_plain_text();
			let syntax = SYNTAX_SET
				.find_syntax_for_file(file_path)
				.unwrap_or_else(|e| {
					log::error!("Could not read the file to detect its syntax: {e}");
					Some(plain_text())
				})
				.unwrap_or_else(plain_text);

			ParseState::new(syntax)
		};

		let theme =
			THEME_SET.themes.get(syntax).unwrap_or_else(|| {
				&THEME_SET.themes["base16-eighties.dark"]
			});
		let highlighter = Highlighter::new(theme);

		let mut syntax_lines: Vec<SyntaxLine> = Vec::new();

		let mut highlight_state =
			HighlightState::new(&highlighter, ScopeStack::new());

		{
			let total_count = text.lines().count();

			let mut buffer = AsyncProgressBuffer::new(
				total_count,
				Duration::from_millis(200),
			);
			params.set_progress(buffer.send_progress())?;
			params.send(AsyncAppNotification::SyntaxHighlighting(
				SyntaxHighlightProgress::Progress,
			))?;

			for (number, line) in text.lines().enumerate() {
				let ops = state
					.parse_line(line, &SYNTAX_SET)
					.map_err(|e| {
						log::error!("syntax error: {:?}", e);
						asyncgit::Error::Generic(
							"syntax error".to_string(),
						)
					})?;
				let iter = RangedHighlightIterator::new(
					&mut highlight_state,
					&ops[..],
					line,
					&highlighter,
				);

				syntax_lines.push(SyntaxLine {
					items: iter
						.map(|(style, _, range)| {
							(style, number, range)
						})
						.collect(),
				});

				if buffer.update(number) {
					params.set_progress(buffer.send_progress())?;
					params.send(
						AsyncAppNotification::SyntaxHighlighting(
							SyntaxHighlightProgress::Progress,
						),
					)?;
				}
			}
		}

		Ok(Self {
			text,
			lines: syntax_lines,
			path: file_path.into(),
		})
	}

	///
	pub fn path(&self) -> &Path {
		&self.path
	}
}

impl<'a> From<&'a SyntaxText> for ratatui::text::Text<'a> {
	fn from(v: &'a SyntaxText) -> Self {
		let mut result_lines: Vec<Line> =
			Vec::with_capacity(v.lines.len());

		for (syntax_line, line_content) in
			v.lines.iter().zip(v.text.lines())
		{
			let mut line_span: Line =
				Vec::with_capacity(syntax_line.items.len()).into();

			for (style, _, range) in &syntax_line.items {
				let item_content = &line_content[range.clone()];
				let item_style = syntact_style_to_tui(style);

				line_span
					.spans
					.push(Span::styled(item_content, item_style));
			}

			result_lines.push(line_span);
		}

		result_lines.into()
	}
}

fn syntact_style_to_tui(style: &Style) -> ratatui::style::Style {
	let mut res = ratatui::style::Style::default().fg(
		ratatui::style::Color::Rgb(
			style.foreground.r,
			style.foreground.g,
			style.foreground.b,
		),
	);

	if style.font_style.contains(FontStyle::BOLD) {
		res = res.add_modifier(ratatui::style::Modifier::BOLD);
	}
	if style.font_style.contains(FontStyle::ITALIC) {
		res = res.add_modifier(ratatui::style::Modifier::ITALIC);
	}
	if style.font_style.contains(FontStyle::UNDERLINE) {
		res = res.add_modifier(ratatui::style::Modifier::UNDERLINED);
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
	syntax: String,
}

impl AsyncSyntaxJob {
	pub fn new(
		content: String,
		path: String,
		syntax: String,
	) -> Self {
		Self {
			state: Arc::new(Mutex::new(Some(JobState::Request((
				content, path,
			))))),
			syntax,
		}
	}

	///
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
	type Notification = AsyncAppNotification;
	type Progress = ProgressPercent;

	fn run(
		&mut self,
		params: RunParams<Self::Notification, Self::Progress>,
	) -> asyncgit::Result<Self::Notification> {
		let mut state_mutex = self.state.lock()?;

		if let Some(state) = state_mutex.take() {
			*state_mutex = Some(match state {
				JobState::Request((content, path)) => {
					let syntax = SyntaxText::new(
						content,
						Path::new(&path),
						&params,
						&self.syntax,
					)?;
					JobState::Response(syntax)
				}
				JobState::Response(res) => JobState::Response(res),
			});
		}

		Ok(AsyncAppNotification::SyntaxHighlighting(
			SyntaxHighlightProgress::Done,
		))
	}
}
