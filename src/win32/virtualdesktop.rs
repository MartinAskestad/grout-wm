use log::info;
use windows::Win32::{
    Foundation::HWND,
    UI::Shell::{IVirtualDesktopManager, VirtualDesktopManager as VirtualDesktopManager_ID},
};

use grout_wm::Result;

pub struct VirtualDesktopManager(IVirtualDesktopManager);

impl VirtualDesktopManager {
    pub fn new() -> Result<Self> {
        use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_ALL};
        info!("Instantiate VirtualDesktopManager");
        let virtual_desktop_managr =
            unsafe { CoCreateInstance(&VirtualDesktopManager_ID, None, CLSCTX_ALL)? };
        Ok(Self(virtual_desktop_managr))
    }

    pub fn is_window_on_current_desktop(&self, hwnd: HWND) -> windows::core::Result<bool> {
        let is_on_desktop = unsafe { self.0.IsWindowOnCurrentVirtualDesktop(hwnd)? };
        Ok(is_on_desktop.as_bool())
    }
}
