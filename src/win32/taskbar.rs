use log::info;
use windows::Win32::{
    Foundation::HWND,
    UI::Shell::{ITaskbarList3, TaskbarList as TaskbarList_ID, THUMBBUTTON},
};

use grout_wm::Result;

pub struct TaskbarList(ITaskbarList3);

impl TaskbarList {
    pub fn new() -> Result<Self> {
        use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_ALL};
        info!("Instantiate TaskbarList");
        let taskbarlist = unsafe { CoCreateInstance(&TaskbarList_ID, None, CLSCTX_ALL)? };
        Ok(Self(taskbarlist))
    }

    pub fn thumb_bar_add_buttons(
        &self,
        hwnd: HWND,
        pbutton: &[THUMBBUTTON],
    ) -> windows::core::Result<()> {
        unsafe { self.0.ThumbBarAddButtons(hwnd, pbutton) }
    }
}
