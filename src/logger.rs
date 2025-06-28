use simplelog::{CombinedLogger, SimpleLogger, WriteLogger, Config, LevelFilter};
use std::fs::File;
use std::path::PathBuf;

pub fn init() -> Result<(), Box<dyn std::error::Error>> {
    let exe_path: PathBuf = std::env::current_exe()?;
    let exe_stem = exe_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("EasyProxy");
    let log_file = format!("{}.log", exe_stem);

    CombinedLogger::init(vec![
        SimpleLogger::new(LevelFilter::Info, Config::default()),
        WriteLogger::new(LevelFilter::Info, Config::default(), File::create(log_file)?),
    ])?;
    Ok(())
}
