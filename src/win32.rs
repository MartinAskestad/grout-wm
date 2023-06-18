use std::mem::{size_of, zeroed};
use windows::{
    w,
    Win32::{
        Foundation::{FALSE, HWND, RECT},
        Graphics::Dwm::{
            DwmGetWindowAttribute, DWMWA_CLOAKED, DWM_CLOAKED_APP, DWM_CLOAKED_INHERITED,
            DWM_CLOAKED_SHELL,
        },
        UI::WindowsAndMessaging::{
            HWND_TOP, SWP_NOACTIVATE,
            GetForegroundWindow,SetWindowPos,
            FindWindowW, GetParent, GetSystemMetrics, GetWindowLongPtrW, GetWindowTextLengthW,
            IsIconic, IsWindowVisible, SystemParametersInfoW, GWL_EXSTYLE, GWL_STYLE,
            SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN,
            SPI_GETWORKAREA, SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS,
        },
    },
};

pub fn is_cloaked(hwnd: HWND) -> bool {
    let mut cloaked: u32 = 0;
    let res = unsafe {
        DwmGetWindowAttribute(
            hwnd,
            DWMWA_CLOAKED,
            (&mut cloaked as *mut u32).cast(),
            size_of::<u32>().try_into().unwrap(),
        )
    };
    match res {
        Ok(_) => matches!(
            cloaked,
            DWM_CLOAKED_APP | DWM_CLOAKED_SHELL | DWM_CLOAKED_INHERITED
        ),
        _ => false,
    }
}

pub fn is_iconic(hwnd: HWND) -> bool {
    unsafe { IsIconic(hwnd).into() }
}

pub fn get_parent(hwnd: HWND) -> HWND {
    unsafe { GetParent(hwnd) }
}

pub fn get_window_style(hwnd: HWND) -> u32 {
    unsafe { u32::try_from(GetWindowLongPtrW(hwnd, GWL_STYLE)).unwrap_or(0) }
}

pub fn get_window_exstyle(hwnd: HWND) -> u32 {
    unsafe { u32::try_from(GetWindowLongPtrW(hwnd, GWL_EXSTYLE)).unwrap_or(0) }
}

pub fn is_window_visible(hwnd: HWND) -> bool {
    unsafe { IsWindowVisible(hwnd).into() }
}

pub fn get_window_text_length(hwnd: HWND) -> i32 {
    unsafe { GetWindowTextLengthW(hwnd) }
}

pub fn get_working_area() -> Result<(i32, i32, i32, i32), &'static str> {
    let hwnd = unsafe { FindWindowW(w!("Shell_TrayWnd"), None) };
    let is_visible = is_window_visible(hwnd);
    if hwnd.0 != 0 && is_visible {
        let mut wa: RECT = unsafe { zeroed() };
        let res = unsafe {
            SystemParametersInfoW(
                SPI_GETWORKAREA,
                0,
                Some(&mut wa as *mut RECT as *mut std::ffi::c_void),
                SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
            )
        };
        if res == FALSE {
            return Err("");
        }
        Ok((wa.left, wa.top, wa.right - wa.left, wa.bottom - wa.top))
    } else {
        Ok((
            unsafe { GetSystemMetrics(SM_XVIRTUALSCREEN) },
            unsafe { GetSystemMetrics(SM_YVIRTUALSCREEN) },
            unsafe { GetSystemMetrics(SM_CXVIRTUALSCREEN) },
            unsafe { GetSystemMetrics(SM_CYVIRTUALSCREEN) },
        ))
    }
}

pub fn get_foreground_window() -> HWND {
    unsafe { GetForegroundWindow() }
}

pub fn set_window_pos(hwnd: HWND, d: (i32, i32, i32, i32)) {
    unsafe { SetWindowPos(hwnd, HWND_TOP, d.0, d.1, d.2, d.3, SWP_NOACTIVATE); }
}
