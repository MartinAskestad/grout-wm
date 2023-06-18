#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::os::raw::c_void;
use windows::Win32::Foundation::{BOOL, HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::UI::Accessibility::HWINEVENTHOOK;
use windows::Win32::UI::WindowsAndMessaging::WM_USER;

mod appwindow;
mod arrange;
mod win32;

macro_rules! any {
    ($xs:expr, $x:expr) => {
        $xs.iter().any(|&x| x.hwnd == $x)
    };
}

macro_rules! has_flag {
    ($value:expr, $flag:expr) => {
        ($value & $flag) == $flag
    };
}

#[derive(Clone, Copy, Debug, Default)]
struct Window {
    hwnd: HWND,
}

struct WM {
    managed_windows: Vec<Window>,
    selected_window: Window,
    sx: i32,
    sy: i32,
    sw: i32,
    sh: i32,
}

impl WM {
    fn new() -> Self {
        let working_area = win32::get_working_area().unwrap();
        WM {
            managed_windows: vec![],
            sx: working_area.0,
            sy: working_area.1,
            sw: working_area.2,
            sh: working_area.3,
            selected_window: Default::default(),
        }
    }

    fn unmanage(&mut self, hwnd: HWND) {
        if any!(self.managed_windows, hwnd) {
            self.managed_windows.retain(|w| w.hwnd != hwnd);
        }
    }

    fn get_window(&mut self, hwnd: HWND) -> Option<Window> {
        self.managed_windows
            .iter()
            .find(|w| w.hwnd == hwnd)
            .copied()
    }

    fn manage(&mut self, hwnd: HWND) -> Option<Window> {
        if let Some(w) = self.get_window(hwnd) {
            Some(w)
        } else {
            let w = Window { hwnd };
            self.managed_windows.push(w);
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
        if p_ok && !any!(self.managed_windows, parent) {
            self.manage(parent);
        }
        let title_len = win32::get_window_text_length(hwnd);
        if title_len == 0 || disabled || no_activate || is_cloaked {
            return false;
        }
        if (parent.0 == 0 && is_visible) || p_ok {
            if !is_tool || parent.0 == 0 || p_ok {
                return true;
            }
            if is_app && parent.0 != 0 {
                return true;
            }
        }
        false
    }

    fn arrange(&mut self) {
        let windows_on_screen: Vec<Window> = self
            .managed_windows
            .clone()
            .into_iter()
            .filter(|w| {
                let min = win32::is_iconic(w.hwnd);
                let vis = win32::is_window_visible(w.hwnd);
                !min && vis
            })
            .collect();
        let n = windows_on_screen.len();
        let ds = arrange::spiral_subdivide((self.sx, self.sy, self.sw, self.sh), n);
        for (w, d) in windows_on_screen.iter().zip(ds.iter()) {
            win32::set_window_pos(w.hwnd, *d);
        }
    }

    fn message_loop(&mut self, hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        use windows::Win32::UI::WindowsAndMessaging::{
            DefWindowProcW, HSHELL_WINDOWACTIVATED, HSHELL_WINDOWCREATED, HSHELL_WINDOWDESTROYED,
            WM_DISPLAYCHANGE,
        };
        match msg {
            WM_UNCLOAKED => {
                let hwnd = HWND(lparam.0);
                let w = self.get_window(hwnd);
                if w.is_none() && self.is_manageable(hwnd) {
                    self.manage(hwnd);
                    self.arrange();
                }
            }
            WM_CLOAKED => {
                let hwnd = HWND(lparam.0);
                self.unmanage(hwnd);
                self.arrange();
            }
            WM_DISPLAYCHANGE => {}
            _ => {
                if msg == unsafe { SHELL_HOOK_ID } {
                    let hwnd = HWND(lparam.0);
                    let w = self.get_window(hwnd);
                    match wparam.0 as u32 & 0x7FFF {
                        HSHELL_WINDOWCREATED => {
                            if w.is_none() && self.is_manageable(hwnd) {
                                self.manage(hwnd);
                                self.arrange();
                            }
                        }
                        HSHELL_WINDOWDESTROYED => {
                            if w.is_some() {
                                self.unmanage(hwnd);
                                self.arrange();
                            }
                        }
                        HSHELL_WINDOWACTIVATED => {
                            if let Some(w) = w {
                                let t = self.selected_window;
                                self.selected_window = w;
                                if t.hwnd.0 != 0 && win32::is_iconic(t.hwnd) {
                                    self.arrange();
                                }
                            } else if self.is_manageable(hwnd) {
                                let w = self.manage(hwnd);
                                self.selected_window = w.unwrap();
                                self.arrange();
                            }
                        }
                        _ => {}
                    }
                }
                return unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) };
            }
        }
        LRESULT(0)
    }
}

static mut MY_HWND: HWND = HWND(0);
static mut SHELL_HOOK_ID: u32 = 0;
const WM_UNCLOAKED: u32 = WM_USER + 0x0001;
const WM_CLOAKED: u32 = WM_USER + 0x0002;

extern "system" fn scan(hwnd: HWND, lparam: LPARAM) -> BOOL {
    use windows::Win32::Foundation::TRUE;
    let wm: &mut WM = unsafe { &mut *(lparam.0 as *mut WM) };
    if wm.is_manageable(hwnd) {
        wm.manage(hwnd);
    }
    TRUE
}

fn enum_windows(wm: &mut WM) {
    use windows::Win32::UI::WindowsAndMessaging::EnumWindows;
    unsafe {
        EnumWindows(Some(scan), LPARAM(wm as *mut WM as isize));
    }
}

unsafe extern "system" fn wnd_event_proc(
    _: HWINEVENTHOOK,
    event: u32,
    hwnd: HWND,
    idobject: i32,
    idchild: i32,
    _: u32,
    _: u32,
) {
    use windows::Win32::UI::WindowsAndMessaging::{
        PostMessageW, CHILDID_SELF, EVENT_OBJECT_CLOAKED, EVENT_OBJECT_UNCLOAKED, OBJID_WINDOW,
    };
    if idobject != OBJID_WINDOW.0 || (idchild as u32) != CHILDID_SELF || hwnd.0 == 0 {
        return;
    }
    if event == EVENT_OBJECT_UNCLOAKED {
        PostMessageW(MY_HWND, WM_UNCLOAKED, WPARAM(0), LPARAM(hwnd.0));
    } else if event == EVENT_OBJECT_CLOAKED {
        PostMessageW(MY_HWND, WM_CLOAKED, WPARAM(0), LPARAM(hwnd.0));
    }
}

extern "system" fn wnd_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    use windows::Win32::UI::WindowsAndMessaging::{
        DefWindowProcW, GetWindowLongPtrW, PostQuitMessage, SetWindowLongPtrW, CREATESTRUCTA,
        GWLP_USERDATA, WM_CREATE, WM_DESTROY,
    };
    if msg == WM_DESTROY {
        unsafe {
            PostQuitMessage(0);
        }
        return LRESULT(0);
    }
    if msg == WM_CREATE {
        let create_struct = lparam.0 as *const CREATESTRUCTA;
        let wm = unsafe { (*create_struct).lpCreateParams as *mut WM };
        unsafe {
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, wm as _);
        }
    }
    let wm = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut WM };
    if !wm.is_null() {
        return unsafe { (*wm).message_loop(hwnd, msg, wparam, lparam) };
    }
    unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
}

fn create_app_window(wm: *mut c_void) -> Result<(HWND, HWINEVENTHOOK), &'static str> {
    use windows::{
        w,
        Win32::{
            Foundation::FALSE,
            System::LibraryLoader::GetModuleHandleA,
            UI::{
                Accessibility::SetWinEventHook,
                WindowsAndMessaging::{
                    CreateWindowExW, LoadCursorW, RegisterClassW, RegisterShellHookWindow,
                    RegisterWindowMessageW, ShowWindow, CW_USEDEFAULT, EVENT_OBJECT_CLOAKED,
                    EVENT_OBJECT_UNCLOAKED, IDC_ARROW, SW_SHOWMINNOACTIVE, WINDOW_EX_STYLE,
                    WINEVENT_OUTOFCONTEXT, WNDCLASSW, WS_OVERLAPPEDWINDOW,
                },
            },
        },
    };
    let instance = unsafe { GetModuleHandleA(None) };
    if instance.is_err() {
        return Err("Could not get instance");
    }
    let window_class = w!("grout-wm.window");
    let wc = WNDCLASSW {
        hCursor: unsafe { LoadCursorW(None, IDC_ARROW).unwrap() },
        hInstance: *instance.as_ref().unwrap(),
        lpszClassName: window_class,
        lpfnWndProc: Some(wnd_proc),
        ..Default::default()
    };
    let reg = unsafe { RegisterClassW(&wc) };
    if reg == 0 {
        return Err("Could not register class");
    }
    let hwnd = unsafe {
        CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            window_class,
            w!("group-wm"),
            WS_OVERLAPPEDWINDOW,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            None,
            None,
            instance.unwrap(),
            Some(wm),
        )
    };
    if hwnd.0 == 0 {
        return Err("Could not create window");
    }
    unsafe {
        ShowWindow(hwnd, SW_SHOWMINNOACTIVE);
    }
    let res = unsafe { RegisterShellHookWindow(hwnd) };
    if res == FALSE {
        return Err("Could not register shell hook window");
    }
    unsafe {
        SHELL_HOOK_ID = RegisterWindowMessageW(w!("SHELLHOOK"));
    }
    let wineventhook = unsafe {
        SetWinEventHook(
            EVENT_OBJECT_CLOAKED,
            EVENT_OBJECT_UNCLOAKED,
            None,
            Some(wnd_event_proc),
            0,
            0,
            WINEVENT_OUTOFCONTEXT,
        )
    };
    if wineventhook.0 == 0 {
        return Err("Can't set win event hook");
    }
    Ok((hwnd, wineventhook))
}

fn main() -> Result<(), &'static str> {
    use windows::Win32::{
        UI::Accessibility::UnhookWinEvent,
        UI::WindowsAndMessaging::{
            DeregisterShellHookWindow, DispatchMessageW, GetMessageW, TranslateMessage, MSG,
        },
    };
    let mut wm = WM::new();
    enum_windows(&mut wm);
    let wm_ptr = &mut wm as *mut _ as *mut c_void;
    let appwindow = appwindow::AppWindow::new()?
        .handle_messages()?
        .cleanup();
    // let (hwnd, wineventhook) = create_app_window(wm_ptr)?;
    // let mut message = MSG::default();
    // unsafe {
    //     while GetMessageW(&mut message, HWND(0), 0, 0).into() {
    //         TranslateMessage(&message);
    //         DispatchMessageW(&message);
    //     }
    // }
    // unsafe {
    //     DeregisterShellHookWindow(hwnd);
    //     UnhookWinEvent(wineventhook);
    // }
    Ok(())
}
