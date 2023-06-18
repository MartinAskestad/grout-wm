use windows::w;
use windows::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, WPARAM},
    UI::{
        Accessibility::{UnhookWinEvent, HWINEVENTHOOK},
        WindowsAndMessaging::{
            CreateWindowExW, DefWindowProcW, DeregisterShellHookWindow, RegisterShellHookWindow,
            CW_USEDEFAULT, WINDOW_EX_STYLE, WNDCLASSW, WS_OVERLAPPEDWINDOW,
        },
    },
};

use crate::win32;

pub struct AppWindow {
    hwnd: HWND,
    wineventhook: HWINEVENTHOOK,
    shell_hook_id: u32,
}

impl AppWindow {
    pub fn new() -> Result<Self, &'static str> {
        let instance_res = win32::get_module_handle();
        if let Ok(instance) = instance_res {
            let windows_class = w!("grout-wm.window");
            let wc = WNDCLASSW {
                hInstance: instance,
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
                    None,
                )
            };
            if hwnd.0 == 0 {
                return Err("Could not create window");
            }
            win32::show_window(hwnd);
            let shell_hook_res = win32::register_shell_hook_window(hwnd);
            if !shell_hook_res {
                return Err("Could not register shell hook window");
            }
            let shell_hook_id = win32::register_window_messagew(w!("SHELLHOOK"));
            let wineventhook = win32::set_win_event_hook(Some(Self::wnd_event_proc));
            Ok(Self {
                hwnd,
                wineventhook,
                shell_hook_id,
            })
        } else {
            return Err("Could not get instance");
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
            UnhookWinEvent(self.wineventhook);
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
        use windows::Win32::UI::WindowsAndMessaging::{
            PostMessageW, CHILDID_SELF, EVENT_OBJECT_CLOAKED, EVENT_OBJECT_UNCLOAKED, OBJID_WINDOW,
        };
        if idobject != OBJID_WINDOW.0 || (idchild as u32) != CHILDID_SELF || hwnd.0 == 0 {
            return;
        }
        if event == EVENT_OBJECT_UNCLOAKED {
            println!("uncloaked");
            // PostMessageW(MY_HWND, WM_UNCLOAKED, WPARAM(0), LPARAM(hwnd.0));
        } else if event == EVENT_OBJECT_CLOAKED {
            println!("cloaked");
            // PostMessageW(MY_HWND, WM_CLOAKED, WPARAM(0), LPARAM(hwnd.0));
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
            // let create_struct = lparam.0 as *const CREATESTRUCTA;
            // let wm = unsafe { (*create_struct).lpCreateParams as *mut WM };
            // unsafe {
            //     SetWindowLongPtrW(hwnd, GWLP_USERDATA, wm as _);
            // }
        }
        // let wm = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut WM };
        // if !wm.is_null() {
        //     return unsafe { (*wm).message_loop(hwnd, msg, wparam, lparam) };
        // }
        unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
    }
}
