use anyhow::Result;
use bugreport::{bugreport, collector::*, format::Markdown};

use crate::get_app_config_path;

pub fn generate_bugreport() -> Result<()> {
    let mut config_file = get_app_config_path()?;
    config_file.push("gitui/");

    bugreport!()
        .info(SoftwareVersion::default())
        .info(OperatingSystem::default())
        .info(CompileTimeInformation::default())
        .info(EnvironmentVariables::list(&["SHELL", "EDITOR"]))
        .info(CommandLine::default())
        .info(FileContent::new(
            "theme.ron",
            config_file.with_file_name("theme.ron"),
        ))
        .info(FileContent::new(
            "key_config.ron",
            config_file.with_file_name("key_config.ron"),
        ))
        .print::<Markdown>();
    Ok(())
}
