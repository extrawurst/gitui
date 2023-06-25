use crate::{
	components::{
		visibility_blocking, CommandBlocking, CommandInfo, Component,
		DrawableComponent, EventState,
	},
	keys::SharedKeyConfig,
	strings,
	ui::{self, style::SharedTheme},
};
use anyhow::{anyhow, bail, Result};
use asyncgit::sync::{
	get_config_string, utils::repo_work_dir, RepoPath,
};
use crossterm::{
	event::Event,
	terminal::{EnterAlternateScreen, LeaveAlternateScreen},
	ExecutableCommand,
};
use ratatui::{
	backend::Backend,
	layout::Rect,
	text::{Line, Span},
	widgets::{Block, BorderType, Borders, Clear, Paragraph},
	Frame,
};
use scopeguard::defer;
use std::ffi::OsStr;
use std::{env, io, path::Path, process::Command};

///
pub struct ExternalEditorComponent {
	visible: bool,
	theme: SharedTheme,
	key_config: SharedKeyConfig,
}

impl ExternalEditorComponent {
	///
	pub fn new(
		theme: SharedTheme,
		key_config: SharedKeyConfig,
	) -> Self {
		Self {
			visible: false,
			theme,
			key_config,
		}
	}

	/// opens file at given `path` in an available editor
	pub fn open_file_in_editor(
		repo: &RepoPath,
		path: &Path,
	) -> Result<()> {
		let work_dir = repo_work_dir(repo)?;

		let path = if path.is_relative() {
			Path::new(&work_dir).join(path)
		} else {
			path.into()
		};

		if !path.exists() {
			bail!("file not found: {:?}", path);
		}

		// so that the output is not messed up when running tests
		if cfg!(not(test)) {
			io::stdout().execute(LeaveAlternateScreen)?;
			defer! {
				io::stdout().execute(EnterAlternateScreen).expect("reset terminal");
			}
		}

		let environment_options = ["GIT_EDITOR", "VISUAL", "EDITOR"];

		let editor = env::var(environment_options[0])
			.ok()
			.or_else(|| {
				get_config_string(repo, "core.editor").ok()?
			})
			.or_else(|| env::var(environment_options[1]).ok())
			.or_else(|| env::var(environment_options[2]).ok())
			.unwrap_or_else(|| String::from("vi"));

		log::trace!("external editor:{}", editor);
		// TODO: proper handling arguments containing whitespaces
		// This does not do the right thing if the input is `editor --something "with spaces"`

		// deal with "editor name with spaces" p1 p2 p3
		// and with "editor_no_spaces" p1 p2 p3
		// does not address spaces in pn
		let mut echars = editor.chars().peekable();

		let first_char = *echars.peek().ok_or_else(|| {
			anyhow!(
				"editor env variable found empty: {}",
				environment_options.join(" or ")
			)
		})?;
		let command: String = if first_char == '\"' {
			echars
				.by_ref()
				.skip(1)
				.take_while(|c| *c != '\"')
				.collect()
		} else {
			echars.by_ref().take_while(|c| *c != ' ').collect()
		};

		let remainder_str = echars.collect::<String>();
		let remainder = remainder_str.split_whitespace();

		let mut args: Vec<&OsStr> =
			remainder.map(OsStr::new).collect();

		args.push(path.as_os_str());

		let exec_result = Command::new(&command)
			.current_dir(&work_dir)
			.args(&args)
			.status();

		if cfg!(windows) {
			// if command failed to run on windows retry as a batch file (.bat, .cmd,...)
			if exec_result.is_err() {
				/*  here args contains the arguments pulled from the configured editor string
					"myeditor --color blue" ->
						args[0] = "--color"
						args[1] = "blue"

					now insert before these
						"/C"
						"myeditor"
				*/

				args.insert(0, OsStr::new("/C"));
				args.insert(1, OsStr::new(&command));
				let exec_result2 = Command::new("cmd")
					.current_dir(work_dir)
					.args(args)
					.status();

				match exec_result2 {
					// failed to start (unlikely as cmd would have to be missing)
					Err(e) => bail!("\"{}\": {}", command, e),

					// ran, did it complete OK?
					Ok(stat) => {
						// no result is treated as arbitrary failure code of 99
						let code = stat.code().unwrap_or(99);
						if code != 0 {
							bail!(
								"\"{}\": cmd.exe returned {}",
								command,
								code
							)
						}
					}
				};
			}
		} else {
			exec_result
				.map_err(|e| anyhow!("\"{}\": {}", command, e))?;
		}

		Ok(())
	}
}

impl DrawableComponent for ExternalEditorComponent {
	fn draw<B: Backend>(
		&self,
		f: &mut Frame<B>,
		_rect: Rect,
	) -> Result<()> {
		if self.visible {
			let txt = Line::from(
				strings::msg_opening_editor(&self.key_config)
					.split('\n')
					.map(|string| {
						Span::raw::<String>(string.to_string())
					})
					.collect::<Vec<Span>>(),
			);

			let area = ui::centered_rect_absolute(25, 3, f.size());
			f.render_widget(Clear, area);
			f.render_widget(
				Paragraph::new(txt)
					.block(
						Block::default()
							.borders(Borders::ALL)
							.border_type(BorderType::Thick)
							.border_style(self.theme.block(true)),
					)
					.style(self.theme.block(true)),
				area,
			);
		}

		Ok(())
	}
}

impl Component for ExternalEditorComponent {
	fn commands(
		&self,
		out: &mut Vec<CommandInfo>,
		force_all: bool,
	) -> CommandBlocking {
		if self.visible && !force_all {
			out.clear();
		}

		visibility_blocking(self)
	}

	fn event(&mut self, _ev: &Event) -> Result<EventState> {
		if self.visible {
			return Ok(EventState::Consumed);
		}

		Ok(EventState::NotConsumed)
	}

	fn is_visible(&self) -> bool {
		self.visible
	}

	fn hide(&mut self) {
		self.visible = false;
	}

	fn show(&mut self) -> Result<()> {
		self.visible = true;

		Ok(())
	}
}
#[cfg(test)]
mod tests {
	use crate::components::ExternalEditorComponent;
	use anyhow::Result;
	use asyncgit::sync::tests::repo_init;
	#[cfg(windows)]
	use asyncgit::sync::utils::read_file;
	use asyncgit::sync::RepoPath;
	use serial_test::serial;
	use std::env;
	use std::fs::File;
	use std::io::Write;
	use tempfile::TempDir;

	fn write_temp_file(
		td: &TempDir,
		file: &str,
		content: &str,
	) -> Result<()> {
		let binding = td.path().join(file);
		let file_path = binding.to_str().unwrap();
		let mut file = File::create(file_path)?;
		file.write_all(content.as_bytes())?;
		Ok(())
	}
	const TEST_FILE_NAME: &str = "test1.txt";
	const TEST_FILE_DATA: &str = "test file data";

	fn setup_repo() -> (TempDir, RepoPath) {
		let (td, repo) = repo_init().unwrap();
		let root = repo.path().parent().unwrap();
		let repo_path: RepoPath =
			root.as_os_str().to_str().unwrap().into();

		// create a dummy file to operate on
		let txt = String::from(TEST_FILE_DATA);
		write_temp_file(&td, TEST_FILE_NAME, &txt).unwrap();
		(td, repo_path)
	}

	// these have to de serialzied because they set env variables to control which editor to use

	#[test]
	#[serial]
	fn editor_missing() {
		let (td, repo_path) = setup_repo();
		let target_file_path = td.path().join(TEST_FILE_NAME);
		env::set_var("GIT_EDITOR", "i_doubt_this_exists");
		let foo = ExternalEditorComponent::open_file_in_editor(
			&repo_path,
			&target_file_path,
		);
		assert!(foo.is_err());
	}

	#[cfg(windows)]
	mod win_test {
		use super::*;
		#[test]
		#[serial]
		fn editor_is_bat() {
			let (td, repo_path) = setup_repo();
			let target_file_path = td.path().join(TEST_FILE_NAME);
			env::set_var("GIT_EDITOR", "testbat");
			let bat = String::from("@echo off\ntype %1 >made.txt");
			write_temp_file(&td, "testbat.bat", &bat).unwrap();

			let runit = ExternalEditorComponent::open_file_in_editor(
				&repo_path,
				&target_file_path,
			);
			assert!(runit.is_ok());

			let echo_file = td.path().join("made.txt");
			let read_text = read_file(echo_file.as_path()).unwrap();

			assert_eq!(
				read_text.lines().next(),
				Some(TEST_FILE_DATA)
			);
		}
		#[test]
		#[serial]
		fn editor_is_bat_ext() {
			let (td, repo_path) = setup_repo();
			let target_file_path = td.path().join(TEST_FILE_NAME);

			env::set_var("GIT_EDITOR", "testbat.bat");

			let bat = String::from("@echo off\ntype %1 >made.txt");
			write_temp_file(&td, "testbat.bat", &bat).unwrap();

			let runit = ExternalEditorComponent::open_file_in_editor(
				&repo_path,
				&target_file_path,
			);
			assert!(runit.is_ok());

			let echo_file = td.path().join("made.txt");
			let read_text = read_file(echo_file.as_path()).unwrap();
			assert_eq!(
				read_text.lines().next(),
				Some(TEST_FILE_DATA)
			);
		}
		#[test]
		#[serial]
		fn editor_is_bat_noext_arg() {
			let (td, repo_path) = setup_repo();
			let target_file_path = td.path().join(TEST_FILE_NAME);

			env::set_var("GIT_EDITOR", "testbat --foo");

			let bat = String::from("@echo off\ntype %2 >made.txt");
			write_temp_file(&td, "testbat.bat", &bat).unwrap();

			let runit = ExternalEditorComponent::open_file_in_editor(
				&repo_path,
				&target_file_path,
			);
			assert!(runit.is_ok());

			let echo_file = td.path().join("made.txt");
			let read_text = read_file(echo_file.as_path()).unwrap();
			assert_eq!(
				read_text.lines().next(),
				Some(TEST_FILE_DATA)
			);
		}
		#[test]
		#[serial]
		fn editor_is_cmd() {
			let (td, repo_path) = setup_repo();
			let target_file_path = td.path().join(TEST_FILE_NAME);
			env::set_var("GIT_EDITOR", "testcmd");
			let bat = String::from("@echo off\ntype %1 >made.txt");
			write_temp_file(&td, "testcmd.cmd", &bat).unwrap();

			let runit = ExternalEditorComponent::open_file_in_editor(
				&repo_path,
				&target_file_path,
			);
			assert!(runit.is_ok());

			let echo_file = td.path().join("made.txt");
			let read_text = read_file(echo_file.as_path()).unwrap();

			assert_eq!(
				read_text.lines().next(),
				Some(TEST_FILE_DATA)
			);
		}
		#[test]
		#[serial]
		fn editor_is_cmd_arg() {
			let (td, repo_path) = setup_repo();
			let target_file_path = td.path().join(TEST_FILE_NAME);
			env::set_var("GIT_EDITOR", "testcmd --bar");
			let bat = String::from("@echo off\ntype %2 >made.txt");
			write_temp_file(&td, "testcmd.cmd", &bat).unwrap();

			let runit = ExternalEditorComponent::open_file_in_editor(
				&repo_path,
				&target_file_path,
			);
			assert!(runit.is_ok());

			let echo_file = td.path().join("made.txt");
			let read_text = read_file(echo_file.as_path()).unwrap();

			assert_eq!(
				read_text.lines().next(),
				Some(TEST_FILE_DATA)
			);
		}
	}
}
