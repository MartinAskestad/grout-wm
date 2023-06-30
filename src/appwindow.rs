use std::os::raw::c_void;
use std::sync::OnceLock;
use windows::w;
use windows::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, WPARAM},
    Graphics::Gdi::{COLOR_WINDOW, HBRUSH},
    UI::{
        Accessibility::{UnhookWinEvent, HWINEVENTHOOK},
        WindowsAndMessaging::{
            CreateWindowExW, DefWindowProcW, DeregisterShellHookWindow, GetWindowLongPtrW,
            PostMessageW, PostQuitMessage, SetWindowLongPtrW, CHILDID_SELF, CREATESTRUCTA,
            CW_USEDEFAULT, EVENT_OBJECT_CLOAKED, EVENT_OBJECT_UNCLOAKED, EVENT_SYSTEM_MINIMIZEEND,
            EVENT_SYSTEM_MINIMIZESTART, GWLP_USERDATA, OBJID_WINDOW, WINDOW_EX_STYLE, WM_CREATE,
            WM_DESTROY, WNDCLASSW, WS_OVERLAPPEDWINDOW,
        },
    },
};

use crate::win32;
use crate::wm::{WM, WM_CLOAKED, WM_MINIMIZEEND, WM_MINIMIZESTART, WM_UNCLOAKED};

static MY_HWND: OnceLock<HWND> = OnceLock::new();

pub struct AppWindow {
    hwnd: HWND,
    cloaked_event_hook: HWINEVENTHOOK,
    minimized_event_hook: HWINEVENTHOOK,
}

impl AppWindow {
    pub fn new(wm: &mut WM) -> Result<Self, &'static str> {
        let instance_res = win32::get_module_handle();
        if let Ok(instance) = instance_res {
            let windows_class = w!("grout-wm.window");
            let wc = WNDCLASSW {
                hInstance: instance,
                hbrBackground: HBRUSH((COLOR_WINDOW.0 + 1) as isize),
                lpszClassName: windows_class,
                lpfnWndProc: Some(Self::wnd_proc),
                ..Default::default()
            };
            if win32::register_class(&wc) == 0 {
                return Err("Could not register class");
            }
            let hwnd = unsafe {
                CreateWindowExW(
                    WINDOW_EX_STYLE::default(),
                    windows_class,
                    w!("grout-wm"),
                    WS_OVERLAPPEDWINDOW,
                    CW_USEDEFAULT,
                    CW_USEDEFAULT,
                    CW_USEDEFAULT,
                    CW_USEDEFAULT,
                    None,
                    None,
                    instance,
                    Some(wm as *mut _ as *mut c_void),
                )
            };
            if hwnd.0 == 0 {
                return Err("Could not create window");
            }
            let _ = MY_HWND.set(hwnd);
            win32::show_window(hwnd);
            wm.manage(hwnd);
            let shell_hook_res = win32::register_shell_hook_window(hwnd);
            if !shell_hook_res {
                return Err("Could not register shell hook window");
            }
            let shell_hook_id = win32::register_window_messagew(w!("SHELLHOOK"));
            wm.set_shell_hook_id(shell_hook_id);
            let cloaked_event_hook = win32::set_win_event_hook(
                EVENT_OBJECT_CLOAKED,
                EVENT_OBJECT_UNCLOAKED,
                Some(Self::wnd_event_proc),
            );
            let minimized_event_hook = win32::set_win_event_hook(
                EVENT_SYSTEM_MINIMIZESTART,
                EVENT_SYSTEM_MINIMIZEEND,
                Some(Self::wnd_event_proc),
            );
            Ok(Self {
                hwnd,
                cloaked_event_hook,
                minimized_event_hook,
            })
        } else {
            Err("Could not get instance")
        }
    }

    pub fn handle_messages(&self) -> Result<&Self, &'static str> {
        use windows::Win32::UI::WindowsAndMessaging::{
            DispatchMessageW, GetMessageW, TranslateMessage, MSG,
        };
        let mut message = MSG::default();
        unsafe {
            while GetMessageW(&mut message, HWND(0), 0, 0).into() {
                TranslateMessage(&message);
                DispatchMessageW(&message);
            }
        }
        Ok(self)
    }

    pub fn cleanup(&self) -> Result<&Self, &'static str> {
        unsafe {
            DeregisterShellHookWindow(self.hwnd);
            UnhookWinEvent(self.cloaked_event_hook);
            UnhookWinEvent(self.minimized_event_hook);
        }
        Ok(self)
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
        if idobject != OBJID_WINDOW.0 || (idchild as u32) != CHILDID_SELF || hwnd.0 == 0 {
            return;
        }
        if let Some(&my_hwnd) = MY_HWND.get() {
            if event == EVENT_SYSTEM_MINIMIZEEND {
                PostMessageW(my_hwnd, WM_MINIMIZEEND, WPARAM(0), LPARAM(hwnd.0));
            }
            if event == EVENT_SYSTEM_MINIMIZESTART {
                PostMessageW(my_hwnd, WM_MINIMIZESTART, WPARAM(0), LPARAM(hwnd.0));
            }
            if event == EVENT_OBJECT_UNCLOAKED {
                PostMessageW(my_hwnd, WM_UNCLOAKED, WPARAM(0), LPARAM(hwnd.0));
            } else if event == EVENT_OBJECT_CLOAKED {
                PostMessageW(my_hwnd, WM_CLOAKED, WPARAM(0), LPARAM(hwnd.0));
            }
        }
    }

    extern "system" fn wnd_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
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
}
