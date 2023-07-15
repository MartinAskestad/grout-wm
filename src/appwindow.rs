use std::{ffi::c_void, sync::OnceLock};

use log::{debug, error, info};
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
                EVENT_SYSTEM_MINIMIZEEND, EVENT_SYSTEM_MINIMIZESTART, EVENT_SYSTEM_MOVESIZEEND,
                EVENT_SYSTEM_MOVESIZESTART, GWLP_USERDATA, OBJID_WINDOW, WINDOW_EX_STYLE, WM_APP,
                WM_CREATE, WM_DESTROY, WM_USER, WNDCLASSW, WS_OVERLAPPEDWINDOW, WM_SYSCOMMAND, SC_RESTORE,
            },
        },
    },
};

use grout_wm::Result;

use crate::{
    win32::{self, load_icon, get_module_handle, register_class, show_window, register_shell_hook_window, register_window_messagew, set_win_event_hook, def_window_proc, set_window_long_ptr, post_quit_message, get_window_long_ptr},
    windowmanager::{
        WindowManager, MSG_CLOAKED, MSG_MINIMIZEEND, MSG_MINIMIZESTART, MSG_MOVESIZEEND,
        MSG_UNCLOAKED, SHELL_HOOK_ID,
    },
};

static MY_HWND: OnceLock<HWND> = OnceLock::new();

pub struct AppWindow {
    hwnd: HWND,
    cloaked_event_hook: HWINEVENTHOOK,
    minimized_event_hook: HWINEVENTHOOK,
    movesize_event_hook: HWINEVENTHOOK,
}


impl AppWindow {
    pub fn new_window(wm: &mut WindowManager) -> Result<Self> {
        let instance = get_module_handle()?;
        let window_class = w!("grout-wm.window");
        let wc = WNDCLASSW {
            hInstance: instance,
            lpszClassName: window_class,
            hIcon: load_icon(instance, w!("appicon"))?,
            hbrBackground: HBRUSH((COLOR_WINDOW.0 + 1) as isize),
            lpfnWndProc: Some(Self::wnd_proc),
            ..Default::default()
        };
        if register_class(&wc) == 0 {
            error!("Could not register class");
            return Err("Could not register class".into());
        }
        let hwnd = unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                window_class,
                w!("grout-wm"),
                WS_OVERLAPPEDWINDOW,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                None,
                None,
                instance,
                Some(wm as *mut _ as *mut c_void)
                )
        };
        if hwnd.0 == 0 {
            error!("Could not create window");
            return Err("Could not create window".into());
        }
        let _ = MY_HWND.set(hwnd);
        Ok(Self {
            hwnd,
            cloaked_event_hook: Default::default (),
            minimized_event_hook: Default::default(),
            movesize_event_hook: Default::default(),
        })
    }

    pub fn show_window(self) -> Result<Self> {
        show_window(self.hwnd);
        Ok(Self {
            hwnd: self.hwnd,
            cloaked_event_hook: self.cloaked_event_hook,
            minimized_event_hook: self.minimized_event_hook,
            movesize_event_hook: self.movesize_event_hook,
        })
    }

    pub fn register_hooks(self) -> Result<Self> {
        let shell_hook_res = register_shell_hook_window(self.hwnd);
        if !shell_hook_res {
            error!("Could not register shell hook window");
            return Err("Could not register shell hook window".into());
        }
        let shell_hook_id = register_window_messagew(w!("SHELLHOOK"));
        let _ = SHELL_HOOK_ID.set(shell_hook_id);
        let cloaked_event_hook = set_win_event_hook(
            EVENT_OBJECT_CLOAKED,
            EVENT_OBJECT_UNCLOAKED,
            Some(Self::wnd_event_proc),
            );
        let minimized_event_hook = set_win_event_hook(
            EVENT_SYSTEM_MINIMIZESTART,
            EVENT_SYSTEM_MINIMIZEEND,
            Some(Self::wnd_event_proc),
            );
        let movesize_event_hook = set_win_event_hook(
            EVENT_SYSTEM_MOVESIZESTART,
            EVENT_SYSTEM_MOVESIZEEND,
            Some(Self::wnd_event_proc),
            );
        Ok(Self {
            hwnd: self.hwnd,
            cloaked_event_hook,
            minimized_event_hook,
            movesize_event_hook,
        })
    }

    pub fn cleanup(&self) -> Self {
        info!("Cleaning up handles");
        unsafe {
            DeregisterShellHookWindow(self.hwnd);
            UnhookWinEvent(self.cloaked_event_hook);
            UnhookWinEvent(self.minimized_event_hook);
            UnhookWinEvent(self.movesize_event_hook);
        }
        Self {
            hwnd: self.hwnd,
            cloaked_event_hook: Default::default(),
            minimized_event_hook: Default::default(),
            movesize_event_hook: Default::default(),
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
            let msg = match event {
                EVENT_OBJECT_CLOAKED => MSG_CLOAKED,
                EVENT_OBJECT_UNCLOAKED => MSG_UNCLOAKED,
                EVENT_SYSTEM_MINIMIZEEND => MSG_MINIMIZEEND,
                EVENT_SYSTEM_MINIMIZESTART => MSG_MINIMIZESTART,
                EVENT_SYSTEM_MOVESIZEEND => MSG_MOVESIZEEND,
                _ => event,
            };
            if msg >= WM_USER || msg < WM_APP {
                win32::post_message(my_hwnd, msg, WPARAM(0), LPARAM(hwnd.0));
            }
        }
    }

    extern "system" fn wnd_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        match msg {
            WM_DESTROY => {
                info!("Received WM_DESTROY message");
                post_quit_message(0);
                LRESULT(0)
            }
            WM_CREATE => {
                info!("Creating application window");
                let create_struct = lparam.0 as *const CREATESTRUCTA;
                let wm = unsafe { (*create_struct).lpCreateParams as *mut WindowManager };
                set_window_long_ptr(hwnd, GWLP_USERDATA, wm as _);
                LRESULT(0)
            }
            WM_SYSCOMMAND => {
                debug!("WM_SYSCOMMAND {:?}\t{}", wparam, SC_RESTORE);
                if wparam.0 as u32 == SC_RESTORE {
                    return LRESULT(0);
                }
                def_window_proc(hwnd, msg, wparam, lparam)
            }
            _ => {
                let wm = get_window_long_ptr(hwnd, GWLP_USERDATA) as *mut WindowManager;
                if !wm.is_null() {
                    return unsafe { (*wm).message_loop(hwnd, msg, wparam, lparam) };
                }
                def_window_proc(hwnd, msg, wparam, lparam)
            }
        }
    }
}
