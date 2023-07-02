use log::info;

use windows::Win32::{
    Foundation::HWND,
    System::Com::{CoInitialize, CoUninitialize, CoCreateInstance, CLSCTX_ALL},
    UI::Shell::{IVirtualDesktopManager, VirtualDesktopManager as VirtualDesktopManager_ID},
};

pub struct VirtualDesktopManager(IVirtualDesktopManager);

impl VirtualDesktopManager {
    pub fn new() -> windows::core::Result<Self> {
        info!("Instanciate VirtualDesktopManager");
        unsafe { CoInitialize(None)?; }
        let virtual_desktop_managr =
            unsafe { CoCreateInstance(&VirtualDesktopManager_ID, None, CLSCTX_ALL)? };
        Ok(Self(virtual_desktop_managr))
    }

    pub fn is_window_on_current_desktop(&self, hwnd: HWND) -> windows::core::Result<bool> {
        let is_on_current_desktop = unsafe { self.0.IsWindowOnCurrentVirtualDesktop(hwnd)? };
        Ok(is_on_current_desktop.as_bool())
    }
}

impl Drop for VirtualDesktopManager {
    fn drop(&mut self) {
        info!("Drop virtual desktop manager");
        unsafe { CoUninitialize(); }
    }
}
