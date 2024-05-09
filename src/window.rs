use crate::win32;
use core::fmt;
use log::error;
use windows::Win32::Foundation::HWND;

#[derive(Clone, Copy)]
pub struct Window(pub HWND);

impl Window {
    pub fn new(hwnd: HWND) -> Self {
        Window(hwnd)
    }

    pub fn is_iconic(&self) -> bool {
        win32::is_iconic(self.0)
    }

    pub fn title(&self) -> String {
        win32::get_window_text(self.0)
    }

    pub fn class_name(&self) -> String {
        win32::get_window_classname(self.0)
    }

    pub fn exstyle(&self) -> u32 {
        win32::get_window_exstyle(self.0)
    }

    pub fn style(&self) -> u32 {
        win32::get_window_style(self.0)
    }

    pub fn process_name(&self) -> String {
        win32::get_exe_filename(self.0).unwrap_or("".to_owned())
    }

    pub fn position(&self) -> windows::Win32::Foundation::RECT {
        let mut rect: windows::Win32::Foundation::RECT = unsafe { std::mem::zeroed() };
        let res =
            unsafe { windows::Win32::UI::WindowsAndMessaging::GetWindowRect(self.0, &mut rect) };
        if res.is_err() {
            error!("GetWindowRect failed: {:?}", res);
        }
        rect
    }
}

impl fmt::Debug for Window {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Window")
            .field("hwnd", &self.0)
            .field("title", &self.title())
            .field("class", &self.class_name())
            .field("minimized", &self.is_iconic())
            .field("ex_style", &self.exstyle())
            .field("style", &self.style())
            .field("process", &self.process_name())
            .finish()
    }
}
