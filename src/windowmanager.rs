use log::{debug, error, info};
use windows::Win32::{
    Foundation::{BOOL, HWND, LPARAM, LRESULT, TRUE, WPARAM},
    UI::WindowsAndMessaging::{
        GW_OWNER, HSHELL_WINDOWCREATED, HSHELL_WINDOWDESTROYED, WM_USER, WS_CHILD, WS_DISABLED,
        WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
    },
};

use crate::{
    arrange::spiral_subdivide,
    config::Config,
    win32::{self, VirtualDesktopManager},
    window::Window,
};
use grout_wm::{any, has_flag, Result};

pub const MSG_UNCLOAKED: u32 = WM_USER;
pub const MSG_CLOAKED: u32 = WM_USER + 0x0001;
pub const MSG_MINIMIZEEND: u32 = WM_USER + 0x0003;
pub const MSG_MINIMIZESTART: u32 = WM_USER + 0x0004;

pub struct WindowManager {
    managed_windows: Vec<Window>,
    working_area: (i32, i32, i32, i32),
    shell_hook_id: u32,
    config: Config,
    virtual_desktop: VirtualDesktopManager,
}

impl WindowManager {
    pub fn new(config: Config) -> Result<Self> {
        info!("Create new instance of window manager");
        let working_area = win32::get_working_area()?;
        info!("Working area is {:?}", working_area);
        Ok(WindowManager {
            managed_windows: Default::default(),
            working_area,
            shell_hook_id: Default::default(),
            config,
            virtual_desktop: VirtualDesktopManager::new()?,
        })
    }

    fn get_window(&mut self, hwnd: HWND) -> Option<Window> {
        self.managed_windows.iter().find(|w| w.0 == hwnd).copied()
    }

    pub fn manage(&mut self, hwnd: HWND) -> Option<Window> {
        if let Some(w) = self.get_window(hwnd) {
            info!("Window already managed {:?}", w);
            Some(w)
        } else {
            let w = Window::new(hwnd);
            self.managed_windows.push(w);
            info!("Manage new window {:?}", w);
            Some(w)
        }
    }

    fn is_manageable(&mut self, hwnd: HWND) -> bool {
        if any!(self.managed_windows, hwnd) {
            return true;
        }
        let style = win32::get_window_style(hwnd);
        let exstyle = win32::get_window_exstyle(hwnd);
        let is_child = has_flag!(style, WS_CHILD.0);
        let is_cloaked = win32::is_cloaked(hwnd);
        let is_disabled = has_flag!(style, WS_DISABLED.0);
        let is_tool = has_flag!(exstyle, WS_EX_TOOLWINDOW.0);
        let is_visible = win32::is_window_visible(hwnd);
        let no_activate = has_flag!(exstyle, WS_EX_NOACTIVATE.0);
        let title = win32::get_window_text(hwnd);
        let class_name = win32::get_window_classname(hwnd);
        let process_name = win32::get_exe_filename(hwnd);
        let title_len = win32::get_window_text_length(hwnd);
        let owner = win32::get_window(hwnd, GW_OWNER);
        if title_len == 0 || is_disabled {
            info!("Window {title} is not suitable to manage");
            return false;
        }
        if let Some(titles) = &self.config.windows_ui_core_corewindow {
            if class_name.contains("Windows.UI.Core.CoreWindow")
                && titles.iter().any(|t| title.contains(t))
            {
                info!("Window {title} is not suitable to manage");
                return false;
            }
        }
        if let Some(classes) = &self.config.class_names {
            if classes.iter().any(|cn| class_name.contains(cn)) {
                info!("Window {title} is not suitable to manage");
                return false;
            }
        }
        if let Some(processes) = &self.config.process_names {
            if let Some(process_name) = process_name {
                if processes.iter().any(|p| process_name.contains(p)) {
                    info!("Window {title} is not suitable to manage");
                    return false;
                }
            }
        }
        let is_app_window = is_visible && !no_activate && !is_child;
        let is_alt_tab_window = !(is_tool || owner.0 != 0);
        let retval = !is_cloaked && is_app_window && is_alt_tab_window;
        info!("Is manageable {retval} - {title}");
        retval
    }

    pub fn set_shell_hook_id(&mut self, shell_hook_id: u32) {
        self.shell_hook_id = shell_hook_id;
    }

    fn unmanage(&mut self, hwnd: HWND) {
        if !any!(self.managed_windows, hwnd) {
            return;
        }
        let is_on_desktop = self
            .virtual_desktop
            .is_window_on_current_desktop(hwnd)
            .unwrap_or(false);
        if is_on_desktop {
            info!("unmanage {:#?}", self.get_window(hwnd));
            self.managed_windows.retain(|w| w.0 != hwnd);
        }
    }

    pub fn arrange(&self) {
        let windows_on_screen: Vec<Window> = self
            .managed_windows
            .clone()
            .into_iter()
            .filter(|w| !w.is_iconic())
            .filter(|w| {
                self.virtual_desktop
                    .is_window_on_current_desktop(w.0)
                    .unwrap_or(false)
            })
            .collect();
        let number_of_windows = windows_on_screen.len();
        let ds = spiral_subdivide(self.working_area, number_of_windows);
        for (w, d) in windows_on_screen.iter().zip(ds.iter()) {
            win32::set_window_pos(w.0, *d);
        }
    }

    pub fn message_loop(
        &mut self,
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        let handle = HWND(lparam.0);
        let managed_window = self.get_window(handle);
        let wmsg = wparam.0 as u32 & 0x7FFF;
        match (msg, wmsg) {
            (MSG_CLOAKED, _) => {
                if managed_window.is_some() {
                    debug!("Cloaked: {managed_window:#?}");
                    self.unmanage(handle);
                    // self.unmanage_or_pause(handle);
                    self.arrange();
                }
            }
            (MSG_UNCLOAKED, _) => {
                if managed_window.is_none() && self.is_manageable(handle) {
                    debug!("{handle:?} is uncloaked");
                    self.manage(handle);
                    self.arrange();
                }
            }
            (MSG_MINIMIZEEND, _) => {
                if let Some(index) = self.managed_windows.iter().position(|&w| w.0 == handle) {
                    let sel = &mut self.managed_windows[index];
                    debug!("Restored: {sel:#?}");
                    self.arrange();
                }
            }
            (MSG_MINIMIZESTART, _) => {
                if managed_window.is_some() {
                    debug!(
                        "Minimized
            {managed_window:#?}"
                    );
                    self.arrange();
                }
            }
            (id, HSHELL_WINDOWCREATED) if id == self.shell_hook_id => {
                if managed_window.is_none() && self.is_manageable(handle) {
                    debug!("{handle:?} is created");
                    self.manage(handle);
                    self.arrange();
                }
            }
            (id, HSHELL_WINDOWDESTROYED) if id == self.shell_hook_id => {
                if managed_window.is_some() {
                    debug!("{handle:?} is destroyed");
                    self.unmanage(handle);
                    // self.unmanage_or_pause(handle);
                    self.arrange();
                }
            }
            _ => return win32::def_window_proc(hwnd, msg, wparam, lparam),
        }
        LRESULT(0)
    }

    pub fn enum_windows(&mut self) -> Result<&mut Self> {
        let self_ptr = LPARAM(self as *mut Self as isize);
        if win32::enum_windows(Some(Self::scan), self_ptr) {
            Ok(self)
        } else {
            error!("Can not enum windows");
            Err("Can not enum windows".into())
        }
    }

    extern "system" fn scan(hwnd: HWND, lparam: LPARAM) -> BOOL {
        let wm: &mut Self = unsafe { &mut *(lparam.0 as *mut Self) };
        if wm.is_manageable(hwnd) {
            wm.manage(hwnd);
        }
        TRUE
    }
}