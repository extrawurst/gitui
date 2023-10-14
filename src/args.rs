use crate::bug_report;
use anyhow::{anyhow, Result};
use asyncgit::sync::{utils::repo_work_dir, RepoPath};
use clap::{Parser, Subcommand};
use simplelog::{Config, LevelFilter, WriteLogger};
use std::{
	fs::{self, File},
	path::PathBuf,
};

pub struct CliArgs {
	pub theme: PathBuf,
	pub repo_path: RepoPath,
	pub notify_watcher: bool,
	pub start_mode: Option<StartMode>,
}

#[derive(Debug, Clone)]
pub enum StartMode {
	BlameFile { path_in_workdir: PathBuf },
	Log { path_in_workdir: Option<PathBuf> },
	Stash,
}

pub fn process_cmdline() -> Result<CliArgs> {
	let args = AppOptions::parse();

	if args.bugreport {
		bug_report::generate_bugreport();
		std::process::exit(0);
	}
	if args.logging {
		setup_logging()?;
	}

	let gitdir = args.directory;

	#[allow(clippy::option_if_let_else)]
	let repo_path = if let Some(w) = args.workdir {
		RepoPath::Workdir { gitdir, workdir: w }
	} else {
		RepoPath::Path(gitdir)
	};

	let cfg_path = get_app_config_path()?;

	let theme = args
		.theme
		.and_then(|arg_theme| {
			let arg_file = cfg_path.join(arg_theme);
			arg_file.is_file().then_some(arg_file)
		})
		.unwrap_or_else(|| cfg_path.join("theme.ron"));

	let start_mode = args
		.command
		.map(|cmd| parse_start_mode(cmd, &repo_path))
		.transpose()?;

	Ok(CliArgs {
		theme,
		repo_path,
		notify_watcher: args.watcher,
		start_mode,
	})
}

#[derive(Parser)]
#[command(
	author,
	version,
	about,
	help_template = "\
{before-help}gitui {version}
{author}
{about}

{usage-heading} {usage}

{all-args}{after-help}
		"
)]
struct AppOptions {
	/// Set the color theme (defaults to theme.ron)
	#[arg(short = 't', long, value_name = "THEME")]
	theme: Option<String>,

	/// Stores logging output into a cache directory
	#[arg(short = 'l', long)]
	logging: bool,

	/// Use notify-based file system watcher instead of tick-based update.
	/// This is more performant, but can cause issues on some platforms. See https://github.com/extrawurst/gitui/blob/master/FAQ.md#watcher for details.
	#[arg(long)]
	watcher: bool,

	/// Generate a bug report
	#[arg(long)]
	bugreport: bool,

	/// Set the git directory
	#[arg(short = 'd', long, default_value = ".", env = "GIT_DIR")]
	directory: PathBuf,

	/// Set the working directory
	#[arg(short = 'w', long, env = "GIT_WORK_TREE")]
	workdir: Option<PathBuf>,

	#[clap(subcommand)]
	command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
	/// Start with a stash view
	Stash,
	/// Show blame view for a file
	Blame { file: PathBuf },
	/// Show history log (optionally for a file)
	Log { file: Option<PathBuf> },
}

fn setup_logging() -> Result<()> {
	let mut path = get_app_cache_path()?;
	path.push("gitui.log");

	println!("Logging enabled. log written to: {path:?}");

	WriteLogger::init(
		LevelFilter::Trace,
		Config::default(),
		File::create(path)?,
	)?;

	Ok(())
}

fn get_app_cache_path() -> Result<PathBuf> {
	let mut path = dirs::cache_dir()
		.ok_or_else(|| anyhow!("failed to find os cache dir."))?;

	path.push("gitui");
	fs::create_dir_all(&path)?;
	Ok(path)
}

pub fn get_app_config_path() -> Result<PathBuf> {
	let mut path = if cfg!(target_os = "macos") {
		dirs::home_dir().map(|h| h.join(".config"))
	} else {
		dirs::config_dir()
	}
	.ok_or_else(|| anyhow!("failed to find os config dir."))?;

	path.push("gitui");
	fs::create_dir_all(&path)?;
	Ok(path)
}

fn parse_start_mode(
	cmd: Command,
	repo_path: &RepoPath,
) -> Result<StartMode> {
	match cmd {
		Command::Stash => Ok(StartMode::Stash),
		Command::Blame { file } => Ok(StartMode::BlameFile {
			path_in_workdir: find_file_in_workdir(file, repo_path)?,
		}),
		Command::Log { file } => Ok(StartMode::Log {
			path_in_workdir: file
				.map(|f| find_file_in_workdir(f, repo_path))
				.transpose()?,
		}),
	}
}

fn find_file_in_workdir(
	file: PathBuf,
	repo_path: &RepoPath,
) -> Result<PathBuf, anyhow::Error> {
	let path = &file;
	let workdir = PathBuf::try_from(repo_work_dir(repo_path)?)?
		.canonicalize()?;

	let make_error = |e: Option<std::io::Error>| {
		let display_path = if path.is_absolute() {
			path.display().to_string()
		} else {
			let dot = PathBuf::from(".");
			dot.canonicalize()
				.unwrap_or(dot)
				.join(path)
				.display()
				.to_string()
		};
		let e = e.map(|e| format!("{e}: ")).unwrap_or_default();
		anyhow::anyhow!(
			"{e}\"{}\" is not in the working directory (\"{}\")",
			display_path,
			workdir.display(),
		)
	};

	let mut work_dir_comp = workdir.components();
	let path_in_workdir: PathBuf = path
		.canonicalize()
		.map_err(|e| make_error(Some(e)))?
		.components()
		.skip_while(|f_comp| {
			Some(f_comp) == work_dir_comp.next().as_ref()
		})
		.collect();

	if work_dir_comp.next().is_some() {
		// workdir components not exhausted
		// this means the file is not in the work dir
		return Err(make_error(None));
	}
	Ok(path_in_workdir)
}
