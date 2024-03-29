#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use log::{error, info, LevelFilter};
use std::env;

use grout_wm::Result;

mod appwindow;
mod config;
mod layout;
mod win32;
mod window;
mod windowmanager;

use crate::appwindow::AppWindow;
use crate::config::Config;
use crate::windowmanager::WindowManager;

fn main() -> Result<()> {
    let mutex_handle = win32::get_mutex().unwrap_or_else(|_e| {
        error!("Can't run multiple instances");
        std::process::exit(1);
    });
    let app_name = env!("CARGO_PKG_NAME");
    let app_version = env!("CARGO_PKG_VERSION");
    let mut log_path = env::temp_dir();
    log_path.push(format!("{}.log", env!("CARGO_BIN_NAME")));
    #[cfg(not(debug_assertions))]
    let _ = simple_logging::log_to_file(log_path, LevelFilter::Info);
    #[cfg(debug_assertions)]
    simple_logging::log_to_stderr(LevelFilter::Debug);
    info!("{} {} - starting", app_name, app_version);
    let _win32 = win32::com::Win32Com::new().unwrap_or_else(|_e| {
        error!("Can not initialize com");
        std::process::exit(1);
    });
    let config = Config::load_default()?.load_or_create_user_config()?;
    let mut binding = WindowManager::new(config)?;
    let wm = binding.enum_windows()?;
    let _appwindow = AppWindow::new_window(wm)?
        .show_window()?
        .register_hooks()?
        .set_thumb_buttons()?
        .handle_messages()?
        .cleanup();
    info!("quitting");
    win32::release_mutex(mutex_handle);
    Ok(())
}
