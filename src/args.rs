use crate::bug_report;
use anyhow::{anyhow, Result};
use clap::{
    crate_authors, crate_description, crate_name, crate_version,
    App as ClapApp, Arg,
};
use simplelog::{Config, LevelFilter, WriteLogger};
use std::{
    env,
    fs::{self, File},
    path::PathBuf,
};

pub struct CliArgs {
    pub theme: PathBuf,
}

pub fn process_cmdline() -> Result<CliArgs> {
    let app = ClapApp::new(crate_name!())
        .author(crate_authors!())
        .version(crate_version!())
        .about(crate_description!())
        .arg(
            Arg::with_name("theme")
                .help("Set the color theme (defaults to theme.ron)")
                .short("t")
                .long("theme")
                .value_name("THEME")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("logging")
                .help("Stores logging output into a cache directory")
                .short("l")
                .long("logging"),
        )
        .arg(
            Arg::with_name("bugreport")
                .help("Generate a bug report")
                .long("bugreport"),
        )
        .arg(
            Arg::with_name("directory")
                .help("Set the working directory")
                .short("d")
                .long("directory")
                .takes_value(true),
        );

    let arg_matches = app.get_matches();
    if arg_matches.is_present("bugreport") {
        bug_report::generate_bugreport();
        std::process::exit(0);
    }
    if arg_matches.is_present("logging") {
        setup_logging()?;
    }
    if arg_matches.is_present("directory") {
        let directory =
            arg_matches.value_of("directory").unwrap_or(".");
        env::set_current_dir(directory)?;
    }
    let arg_theme =
        arg_matches.value_of("theme").unwrap_or("theme.ron");
    if get_app_config_path()?.join(arg_theme).is_file() {
        Ok(CliArgs {
            theme: get_app_config_path()?.join(arg_theme),
        })
    } else {
        Ok(CliArgs {
            theme: get_app_config_path()?.join("theme.ron"),
        })
    }
}

fn setup_logging() -> Result<()> {
    let mut path = get_app_cache_path()?;
    path.push("gitui.log");

    let _ = WriteLogger::init(
        LevelFilter::Trace,
        Config::default(),
        File::create(path)?,
    );

    Ok(())
}

fn get_app_cache_path() -> Result<PathBuf> {
    let mut path = dirs_next::cache_dir()
        .ok_or_else(|| anyhow!("failed to find os cache dir."))?;

    path.push("gitui");
    fs::create_dir_all(&path)?;
    Ok(path)
}

pub fn get_app_config_path() -> Result<PathBuf> {
    let mut path = if cfg!(target_os = "macos") {
        dirs_next::home_dir().map(|h| h.join(".config"))
    } else {
        dirs_next::config_dir()
    }
    .ok_or_else(|| anyhow!("failed to find os config dir."))?;

    path.push("gitui");
    fs::create_dir_all(&path)?;
    Ok(path)
}
