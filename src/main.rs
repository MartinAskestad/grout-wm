#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use log::{info, LevelFilter};
use std::env;

mod appwindow;
mod arrange;
mod win32;
mod window;
mod wm;

use crate::appwindow::AppWindow;
use crate::wm::WM;

fn main() -> Result<(), &'static str> {
    let app_name = env!("CARGO_PKG_NAME");
    let app_version = env!("CARGO_PKG_VERSION");
    let mut log_path = env::temp_dir();
    log_path.push("grout-wm.log");
    #[cfg(not(debug_assertions))]
    let _ = simple_logging::log_to_file(log_path, LevelFilter::Info);
    #[cfg(debug_assertions)]
    simple_logging::log_to_stderr(LevelFilter::Debug);
    info!("{} {} - starting", app_name, app_version);
    let mut binding = WM::new()?;
    let wm = binding.enum_windows()?;
    let _appwindow = AppWindow::new(wm)?.handle_messages()?.cleanup();
    info!("quitting");
    Ok(())
}
