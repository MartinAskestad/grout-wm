use std::ffi::c_void;
use std::mem::{size_of, zeroed};
use windows::Win32::{
    Foundation::{BOOL, HWND, RECT, TRUE},
    Graphics::Dwm::{
        DwmGetWindowAttribute, DwmInvalidateIconicBitmaps, DwmSetWindowAttribute, DWMWA_CLOAKED,
        DWMWA_EXTENDED_FRAME_BOUNDS, DWMWINDOWATTRIBUTE, DWM_CLOAKED_APP, DWM_CLOAKED_INHERITED,
        DWM_CLOAKED_SHELL,
    },
    UI::WindowsAndMessaging::GetWindowRect,
};

pub fn get_window_extended_frame_bounds(hwnd: HWND) -> RECT {
    let mut rect: RECT = unsafe { zeroed() };
    let mut frame: RECT = unsafe { zeroed() };
    unsafe {
        GetWindowRect(hwnd, &mut rect);
        let _ = DwmGetWindowAttribute(
            hwnd,
            DWMWA_EXTENDED_FRAME_BOUNDS,
            &mut frame as *mut RECT as *mut c_void,
            size_of::<RECT>().try_into().unwrap(),
        );
    }
    RECT {
        left: frame.left - rect.left,
        top: frame.top - rect.top,
        right: frame.right - rect.right,
        bottom: frame.bottom - rect.bottom,
    }
}

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

pub fn set_window_attribute(
    hwnd: HWND,
    dwattribute: DWMWINDOWATTRIBUTE,
) -> windows::core::Result<()> {
    let is_true: BOOL = TRUE;
    unsafe {
        DwmSetWindowAttribute(
            hwnd,
            dwattribute,
            &is_true as *const BOOL as *const _,
            size_of::<BOOL>() as u32,
        )
    }
}

pub fn invalidate_iconic_bitmaps(hwnd: HWND) -> windows::core::Result<()> {
    unsafe { DwmInvalidateIconicBitmaps(hwnd) }
}
