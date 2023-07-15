use log::info;

pub struct Win32Com;

impl Win32Com {
    pub fn new() -> grout_wm::Result<Self> {
        use windows::Win32::System::Com::CoInitialize;
        info!("Initialize COM");
        unsafe {
            CoInitialize(None)?;
        }
        Ok(Win32Com)
    }
}

impl Drop for Win32Com {
    fn drop(&mut self) {
        use windows::Win32::System::Com::CoUninitialize;
        info!("Uninitializing COM");
        unsafe {
            CoUninitialize();
        }
    }
}
