use std::{ffi::c_void, sync::OnceLock};

use log::{error, info};
use windows::{
    w,
    Win32::{
        Foundation::{HWND, LPARAM, LRESULT, WPARAM},
        Graphics::Gdi::{COLOR_WINDOW, HBRUSH},
        UI::{
            Accessibility::{UnhookWinEvent, HWINEVENTHOOK},
            WindowsAndMessaging::{
                CreateWindowExW, DeregisterShellHookWindow, CHILDID_SELF, CREATESTRUCTA,
                CW_USEDEFAULT, EVENT_OBJECT_CLOAKED, EVENT_OBJECT_UNCLOAKED,
                EVENT_SYSTEM_MINIMIZEEND, EVENT_SYSTEM_MINIMIZESTART, GWLP_USERDATA, OBJID_WINDOW,
                WINDOW_EX_STYLE, WM_CREATE, WM_DESTROY, WNDCLASSW, WS_OVERLAPPEDWINDOW,
            },
        },
    },
};

use grout_wm::Result;

use crate::{
    win32,
    windowmanager::{WindowManager, WM_CLOAKED, WM_MINIMIZEEND, WM_MINIMIZESTART, WM_UNCLOAKED},
};

static MY_HWND: OnceLock<HWND> = OnceLock::new();

pub struct AppWindow {
    hwnd: HWND,
    cloaked_event_hook: HWINEVENTHOOK,
    minimized_event_hook: HWINEVENTHOOK,
}

impl AppWindow {
    pub fn new(wm: &mut WindowManager) -> Result<Self> {
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
                error!("Could not register class");
                return Err("Could not register class".into());
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
                error!("Could not create window");
                return Err("Could not create window".into());
            }
            let _ = MY_HWND.set(hwnd);
            win32::show_window(hwnd);
            wm.manage(hwnd);
            wm.arrange();
            let shell_hook_res = win32::register_shell_hook_window(hwnd);
            if !shell_hook_res {
                error!("Could not register shell hook window");
                return Err("Could not register shell hook window".into());
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
            error!("Could not get instace");
            Err("Could not get instance".into())
        }
    }

    pub fn handle_messages(&self) -> Result<&Self> {
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

    pub fn cleanup(&self) -> Result<&Self> {
        info!("Cleaning up handles");
        unsafe {
            DeregisterShellHookWindow(self.hwnd);
            UnhookWinEvent(self.cloaked_event_hook);
            UnhookWinEvent(self.minimized_event_hook);
        }
        Ok(self)
    }

    extern "system" fn wnd_event_proc(
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
                win32::post_message(my_hwnd, WM_MINIMIZEEND, WPARAM(0), LPARAM(hwnd.0));
            }
            if event == EVENT_SYSTEM_MINIMIZESTART {
                win32::post_message(my_hwnd, WM_MINIMIZESTART, WPARAM(0), LPARAM(hwnd.0));
            }
            if event == EVENT_OBJECT_UNCLOAKED {
                win32::post_message(my_hwnd, WM_UNCLOAKED, WPARAM(0), LPARAM(hwnd.0));
            } else if event == EVENT_OBJECT_CLOAKED {
                win32::post_message(my_hwnd, WM_CLOAKED, WPARAM(0), LPARAM(hwnd.0));
            }
        }
    }

    extern "system" fn wnd_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        if msg == WM_DESTROY {
            info!("Received WM_DESTROY message");
            win32::post_quit_message(0);
            return LRESULT(0);
        }
        if msg == WM_CREATE {
            info!("Creating application window");
            let create_struct = lparam.0 as *const CREATESTRUCTA;
            let wm = unsafe { (*create_struct).lpCreateParams as *mut WindowManager };
            win32::set_window_long_ptr(hwnd, GWLP_USERDATA, wm as _);
        }
        let wm = win32::get_window_long_ptr(hwnd, GWLP_USERDATA) as *mut WindowManager;
        if !wm.is_null() {
            return unsafe { (*wm).message_loop(hwnd, msg, wparam, lparam) };
        }
        win32::def_window_proc(hwnd, msg, wparam, lparam)
    }
}
