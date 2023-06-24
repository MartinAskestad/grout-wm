#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod appwindow;
mod arrange;
mod win32;
mod window;
mod wm;

use crate::appwindow::AppWindow;
use crate::wm::WM;

fn main() -> Result<(), &'static str> {
    let mut binding = WM::new()?;
    let wm = binding.enum_windows()?;
    let _appwindow = AppWindow::new(wm)?.handle_messages()?.cleanup();
    Ok(())
}
