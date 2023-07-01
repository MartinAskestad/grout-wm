use crate::arrange::spiral_subdivide;
use crate::config::Config;
use crate::win32;
use crate::window::Window;
use log::{debug, error, info};
use windows::Win32::{
    Foundation::{BOOL, HWND, LPARAM, LRESULT, TRUE, WPARAM},
    UI::WindowsAndMessaging::{
        DefWindowProcW, HSHELL_WINDOWCREATED, HSHELL_WINDOWDESTROYED,
        WM_USER,
    },
};

macro_rules! any {
    ($xs:expr, $x:expr) => {
        $xs.iter().any(|&x| x.hwnd == $x)
    };
}

macro_rules! has_flag {
    ($value:expr, $flag:expr) => {
        ($value & $flag) != 0
    };
}

pub const WM_UNCLOAKED: u32 = WM_USER + 0x0001;
pub const WM_CLOAKED: u32 = WM_USER + 0x0002;
pub const WM_MINIMIZEEND: u32 = WM_USER + 0x0004;
pub const WM_MINIMIZESTART: u32 = WM_USER + 0x0008;

pub struct WM {
    managed_windows: Vec<Window>,
    working_area: (i32, i32, i32, i32),
    shell_hook_id: u32,
    config: Config,
}

impl WM {
    pub fn new(config: Config) -> Result<Self, &'static str> {
        let working_area = win32::get_working_area()?;
        info!("Screen size is {:?}", working_area);
        Ok(WM {
            managed_windows: Default::default(),
            working_area,
            shell_hook_id: Default::default(),
            config,
        })
    }

    fn get_window(&mut self, hwnd: HWND) -> Option<Window> {
        self.managed_windows
            .iter()
            .find(|w| w.hwnd == hwnd)
            .copied()
    }

    pub fn manage(&mut self, hwnd: HWND) -> Option<Window> {
        if let Some(w) = self.get_window(hwnd) {
            info!("manage {:?}", w);
            Some(w)
        } else {
            let w = Window::new(hwnd);
            self.managed_windows.push(w);
            info!("manage {:?}", w);
            Some(w)
        }
    }

    fn is_manageable(&mut self, hwnd: HWND) -> bool {
        use windows::Win32::UI::WindowsAndMessaging::{
            WS_DISABLED, WS_EX_APPWINDOW, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
        };
        if hwnd.0 == 0 {
            return false;
        }
        if any!(self.managed_windows, hwnd) {
            return true;
        }
        let parent = win32::get_parent(hwnd);
        let p_ok = parent.0 != 0 && self.is_manageable(parent);
        let style = win32::get_window_style(hwnd);
        let exstyle = win32::get_window_exstyle(hwnd);
        let is_tool = has_flag!(exstyle, WS_EX_TOOLWINDOW.0);
        let disabled = has_flag!(style, WS_DISABLED.0);
        let is_app = has_flag!(exstyle, WS_EX_APPWINDOW.0);
        let no_activate = has_flag!(exstyle, WS_EX_NOACTIVATE.0);
        let is_visible = win32::is_window_visible(hwnd);
        let is_cloaked = win32::is_cloaked(hwnd);
        let title = win32::get_window_text(hwnd);
        let class_name = win32::get_window_classname(hwnd);
        let process_name = win32::get_exe_filename(hwnd);
        if p_ok && !any!(self.managed_windows, parent) {
            self.manage(parent);
        }
        debug!("is_manageable: {}, hwnd:{:?}, visible: {}, parent: {:?}, parent ok: {}, is tool: {}, app: {}, no activate: {}", title, hwnd, is_visible, parent, p_ok, is_tool, is_app, no_activate);
        let title_len = win32::get_window_text_length(hwnd);
        if title_len == 0 || disabled || no_activate || is_cloaked {
            return false;
        }
        if let Some(titles) = &self.config.windows_ui_core_corewindow {
            if class_name.contains("Windows.UI.Core.CoreWindow")
                && titles.iter().any(|t| title.contains(t))
            {
                return false;
            }
        }
        if let Some(classes) = &self.config.class_names {
            if classes.iter().any(|cn| class_name.contains(cn)) {
                return false;
            }
        }
        if let Some(processes) = &self.config.process_names {
            if let Some(process_name) = process_name {
                if processes.iter().any(|p| process_name.contains(p)) {
                    return false;
                }
            }
        }
        if (parent.0 == 0 && is_visible) || p_ok {
            if !is_tool || parent.0 == 0 || p_ok {
                debug!("{:?}", Window::new(hwnd));
                return true;
            }
            if is_app && parent.0 != 0 {
                debug!("{:?}", Window::new(hwnd));
                return true;
            }
        }
        false
    }

    pub fn set_shell_hook_id(&mut self, shell_hook_id: u32) {
        self.shell_hook_id = shell_hook_id;
    }

    fn unmanage(&mut self, hwnd: HWND) {
        if any!(self.managed_windows, hwnd) {
            info!("unmanage {hwnd:?}");
            self.managed_windows.retain(|w| w.hwnd != hwnd)
        }
    }

    pub fn arrange(&self) {
        let windows_on_screen: Vec<Window> = self
            .managed_windows
            .clone()
            .into_iter()
            .filter(|w| {
                let min = win32::is_iconic(w.hwnd);
                // let visible = win32::is_window_visible(w.hwnd);
                !min //&& visible
            })
            .collect();
        let number_of_windows = windows_on_screen.len();
        let ds = spiral_subdivide(self.working_area, number_of_windows);
        for (w, d) in windows_on_screen.iter().zip(ds.iter()) {
            win32::set_window_pos(w.hwnd, *d);
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
            (WM_CLOAKED, _) => {
                if managed_window.is_some() {
                    debug!("{handle:?} is cloaked");
                    self.unmanage(handle);
                    self.arrange();
                }
            }
            (WM_UNCLOAKED, _) => {
                if managed_window.is_none() && self.is_manageable(handle) {
                    debug!("{handle:?} is uncloaked");
                    self.manage(handle);
                    self.arrange();
                }
            }
            (WM_MINIMIZEEND, _) => {
                if let Some(index) = self.managed_windows.iter().position(|&w| w.hwnd == handle) {
                    let sel = &mut self.managed_windows[index];
                    debug!("{:?} is restored", sel.hwnd);
                    sel.minimized = false;
                    self.arrange();
                }
            }
            (WM_MINIMIZESTART, _) => {
                if managed_window.is_some() {
                    if let Some(index) = self.managed_windows.iter().position(|&w| w.hwnd == handle)
                    {
                        let t = &mut self.managed_windows[index];
                        debug!("{:?} is is minimized", t.hwnd);
                        t.minimized = win32::is_iconic(t.hwnd);
                        if t.minimized {
                            self.arrange();
                        }
                    }
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
                    self.arrange();
                }
            }
            // (id, HSHELL_WINDOWACTIVATED) if id == self.shell_hook_id => {
            //     if let Some(c) = managed_window {
            //         self.set_selected(c.hwnd);
            //     }
            // }
            _ => {
                return unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) };
            }
        }
        LRESULT(0)
    }

    pub fn enum_windows(&mut self) -> Result<&mut Self, &'static str> {
        let self_ptr = LPARAM(self as *mut Self as isize);
        if win32::enum_windows(Some(Self::scan), self_ptr) {
            Ok(self)
        } else {
            error!("Can not enum windows");
            Err("Can not enum windows")
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
