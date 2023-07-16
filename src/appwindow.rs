use std::{
    ffi::{c_uchar, c_void},
    sync::OnceLock,
};

use log::{debug, error, info};
use windows::{
    w,
    Win32::{
        Foundation::{HWND, LPARAM, LRESULT, WPARAM},
        Graphics::{
            Dwm::{DWMWA_FORCE_ICONIC_REPRESENTATION, DWMWA_HAS_ICONIC_BITMAP},
            Gdi::{BITMAPINFO, BITMAPINFOHEADER, COLOR_WINDOW, HBRUSH},
        },
        UI::{
            Accessibility::{UnhookWinEvent, HWINEVENTHOOK},
            WindowsAndMessaging::{
                CreateWindowExW, DeregisterShellHookWindow, CHILDID_SELF, CREATESTRUCTA,
                CW_USEDEFAULT, EVENT_OBJECT_CLOAKED, EVENT_OBJECT_UNCLOAKED,
                EVENT_SYSTEM_MINIMIZEEND, EVENT_SYSTEM_MINIMIZESTART, EVENT_SYSTEM_MOVESIZEEND,
                EVENT_SYSTEM_MOVESIZESTART, GWLP_USERDATA, OBJID_WINDOW, SC_RESTORE,
                WINDOW_EX_STYLE, WM_APP, WM_CREATE, WM_DESTROY, WM_DWMSENDICONICTHUMBNAIL,
                WM_QUERYOPEN, WM_SYSCOMMAND, WM_USER, WNDCLASSW, WS_OVERLAPPEDWINDOW,
            },
        },
    },
};

use grout_wm::Result;

use crate::{
    win32::{
        self, def_window_proc, get_module_handle, get_window_long_ptr, get_working_area, load_icon,
        post_quit_message, register_class, register_shell_hook_window, register_window_messagew,
        set_win_event_hook, set_window_long_ptr, show_window,
    },
    windowmanager::{
        WindowManager, MSG_CLOAKED, MSG_MINIMIZEEND, MSG_MINIMIZESTART, MSG_MOVESIZEEND,
        MSG_UNCLOAKED, SHELL_HOOK_ID,
    },
};

macro_rules! LOWORD {
    ($w:expr) => {
        $w & 0xFFFF
    };
}

macro_rules! HIWORD {
    ($w:expr) => {
        ($w >> 16) & 0xFFFF
    };
}

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
                Some(wm as *mut _ as *mut c_void),
            )
        };
        if hwnd.0 == 0 {
            error!("Could not create window");
            return Err("Could not create window".into());
        }
        let _ = MY_HWND.set(hwnd);
        Ok(Self {
            hwnd,
            cloaked_event_hook: Default::default(),
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
                let _ = win32::dwm::set_window_attribute(hwnd, DWMWA_HAS_ICONIC_BITMAP);
                let _ = win32::dwm::set_window_attribute(hwnd, DWMWA_FORCE_ICONIC_REPRESENTATION);
                let _ = win32::dwm::invalidate_iconic_bitmaps(hwnd);
                LRESULT(0)
            }
            WM_DWMSENDICONICTHUMBNAIL => {
                debug!("WM_DWMSENDICONICTHUMBNAIL");
                let width = HIWORD!(lparam.0);
                let height = LOWORD!(lparam.0);
                let _ = set_screenshot_as_iconic_thumbnail(hwnd, width, height);
                LRESULT(0)
            }
            WM_SYSCOMMAND => {
                debug!("WM_SYSCOMMAND {:?}\t{}", wparam, SC_RESTORE);
                if wparam.0 as u32 == SC_RESTORE {
                    return LRESULT(0);
                }
                def_window_proc(hwnd, msg, wparam, lparam)
            }
            WM_QUERYOPEN => LRESULT(0),
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

fn set_screenshot_as_iconic_thumbnail(hwnd: HWND, thumb_w: isize, thumb_h: isize) -> Result<()> {
    let working_area = get_working_area()?;
    let w = working_area.right - working_area.left;
    let h = working_area.bottom - working_area.top;
    let hdc_mem = unsafe { windows::Win32::Graphics::Gdi::CreateCompatibleDC(None) };
    if !hdc_mem.is_invalid() {
        let mut bmi: BITMAPINFO = unsafe { std::mem::zeroed() };
        bmi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
        bmi.bmiHeader.biWidth = thumb_w as i32;
        bmi.bmiHeader.biHeight = thumb_h as i32;
        bmi.bmiHeader.biPlanes = 1;
        bmi.bmiHeader.biBitCount = 32;
        let mut pb_ds = std::ptr::null_mut::<Vec<c_uchar>>();
        let hbm_res = unsafe {
            windows::Win32::Graphics::Gdi::CreateDIBSection(
                hdc_mem,
                &bmi as *const _ as *const _,
                windows::Win32::Graphics::Gdi::DIB_RGB_COLORS,
                &mut pb_ds as *mut _ as *mut _,
                None,
                0,
            )
        };
        if let Ok(hbitmap) = hbm_res {
            let hscreen = unsafe { windows::Win32::Graphics::Gdi::GetDC(None) };
            let hdc = unsafe { windows::Win32::Graphics::Gdi::CreateCompatibleDC(hscreen) };
            unsafe {
                windows::Win32::Graphics::Gdi::SetStretchBltMode(
                    hdc,
                    windows::Win32::Graphics::Gdi::HALFTONE,
                )
            };
            let old_obj = unsafe { windows::Win32::Graphics::Gdi::SelectObject(hdc, hbitmap) };
            let _bret = unsafe {
                windows::Win32::Graphics::Gdi::StretchBlt(
                    hdc,
                    0,
                    0,
                    thumb_w as i32,
                    thumb_h as i32,
                    hscreen,
                    0,
                    0,
                    w,
                    h,
                    windows::Win32::Graphics::Gdi::SRCCOPY,
                )
            };
            let dwm_res =
                unsafe { windows::Win32::Graphics::Dwm::DwmSetIconicThumbnail(hwnd, hbitmap, 0) };
            if let Err(e) = dwm_res {
                dbg!(e);
            }
            // cleanup
            unsafe {
                windows::Win32::Graphics::Gdi::SelectObject(hdc, old_obj);
                windows::Win32::Graphics::Gdi::DeleteDC(hdc);
                windows::Win32::Graphics::Gdi::ReleaseDC(None, hscreen);
                windows::Win32::Graphics::Gdi::DeleteObject(hbitmap);
            }
        }
    }
    Ok(())
}
