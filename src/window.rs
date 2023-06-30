use std::fmt;
use windows::Win32::Foundation::HWND;

use crate::win32;

#[derive(Clone, Copy)]
pub struct Window {
    pub hwnd: HWND,
    pub minimized: bool,
    pub selected: bool,
}

impl Window {
    pub fn new(hwnd: HWND) -> Self {
        Window {
            hwnd,
            minimized: Default::default(),
            selected: Default::default(),
        }
    }
}

impl fmt::Debug for Window {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Window")
            .field("hwnd", &self.hwnd)
            .field("title", &win32::get_window_text(self.hwnd))
            .field("class", &win32::get_window_classname(self.hwnd))
            .field("minimized", &self.minimized)
            .field("parent", &win32::get_parent(self.hwnd))
            .field("ex_style", &win32::get_window_exstyle(self.hwnd))
            .field("style", &win32::get_window_style(self.hwnd))
            .field("process", &win32::get_exe_filename(self.hwnd))
            .finish()
    }
}
