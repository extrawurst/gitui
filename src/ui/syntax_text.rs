use std::{ffi::OsStr, ops::Range, path::Path};
use syntect::{
    highlighting::{
        HighlightState, Highlighter, RangedHighlightIterator, Style,
        ThemeSet,
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
}

impl SyntaxText {
    pub fn new(text: String, file_path: &Path) -> Self {
        //TODO: lazy load
        let ps = SyntaxSet::load_defaults_nonewlines();
        let ts = ThemeSet::load_defaults();
        // log::debug!(
        //     "syntaxes: {:?}",
        //     ps.syntaxes()
        //         .iter()
        //         .map(|s| s.name.clone())
        //         .collect::<Vec<_>>()
        // );

        let mut state = {
            let syntax = file_path
                .extension()
                .and_then(OsStr::to_str)
                .map_or_else(
                    || {
                        ps.find_syntax_by_path(
                            file_path.to_str().unwrap_or_default(),
                        )
                    },
                    |ext| ps.find_syntax_by_extension(ext),
                );

            ParseState::new(
                syntax.unwrap_or_else(|| ps.find_syntax_plain_text()),
            )
        };

        let highlighter =
            Highlighter::new(&ts.themes["base16-eighties.dark"]);

        let mut syntax_lines: Vec<SyntaxLine> = Vec::new();

        let mut highlight_state =
            HighlightState::new(&highlighter, ScopeStack::new());

        for (number, line) in text.lines().enumerate() {
            let ops = state.parse_line(line, &ps);
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
        }
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
    tui::style::Style::default().fg(tui::style::Color::Rgb(
        style.foreground.r,
        style.foreground.g,
        style.foreground.b,
    ))
}
