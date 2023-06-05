use std::io::Error;
use windows::Win32::Foundation::{BOOL, HWND, LPARAM};

macro_rules! any {
    ($xs:expr, $x:expr) => {
        $xs.iter().any(|&x| x == $x)
    };
}

macro_rules! has_flag {
    ($value:expr, $flag:expr) => {
        ($value & $flag) == $flag
    };
}

fn manage(hwnd: HWND, clients: &mut Vec<HWND>) {
    if any!(clients, hwnd) {
        return;
    }
    clients.push(hwnd);
}

fn is_cloaked(hwnd: HWND) -> bool {
    use std::mem::size_of;
    use windows::Win32::Graphics::Dwm::{
        DwmGetWindowAttribute, DWMWA_CLOAKED, DWM_CLOAKED_APP, DWM_CLOAKED_INHERITED,
        DWM_CLOAKED_SHELL,
    };
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

fn is_manageable(hwnd: HWND, clients: &mut Vec<HWND>) -> bool {
    use windows::Win32::UI::WindowsAndMessaging::{
        GetParent, GetWindowLongPtrW, GetWindowTextLengthW, IsWindowVisible, GWL_EXSTYLE,
        GWL_STYLE, WS_DISABLED, WS_EX_APPWINDOW, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
    };
    if hwnd.0 == 0 {
        return false;
    }
    if any!(clients, hwnd) {
        return true;
    }
    let parent = unsafe { GetParent(hwnd) };
    let p_ok = parent.0 != 0 && is_manageable(parent, clients);
    let style = u32::try_from(unsafe { GetWindowLongPtrW(hwnd, GWL_STYLE) }).unwrap_or(0);
    let exstyle = u32::try_from(unsafe { GetWindowLongPtrW(hwnd, GWL_EXSTYLE) }).unwrap_or(0);
    let is_tool = has_flag!(exstyle, WS_EX_TOOLWINDOW.0);
    let disabled = has_flag!(style, WS_DISABLED.0);
    let is_app = has_flag!(exstyle, WS_EX_APPWINDOW.0);
    let no_activate = has_flag!(exstyle, WS_EX_NOACTIVATE.0);
    let is_visible: bool = unsafe { IsWindowVisible(hwnd) }.into();
    let is_cloaked = is_cloaked(hwnd);
    if p_ok && !any!(clients, parent) {
        manage(parent, clients);
    }
    let title_len = unsafe { GetWindowTextLengthW(hwnd) };
    if title_len == 0 || disabled || no_activate || is_cloaked {
        return false;
    }
    if (parent.0 == 0 && is_visible) || p_ok {
        if !is_tool && parent.0 == 0 || (is_tool && p_ok) {
            return true;
        }
        if is_app && parent.0 != 0 {
            return true;
        }
    }
    false
}

extern "system" fn scan(hwnd: HWND, lparam: LPARAM) -> BOOL {
    use windows::Win32::Foundation::TRUE;
    let clients: &mut Vec<HWND> = unsafe { &mut *(lparam.0 as *mut Vec<HWND>) };
    if is_manageable(hwnd, clients) {
        manage(hwnd, clients);
    }
    TRUE
}

fn enum_windows(clients: &mut Vec<HWND>) {
    use windows::Win32::UI::WindowsAndMessaging::EnumWindows;
    unsafe {
        EnumWindows(Some(scan), LPARAM(clients as *mut Vec<HWND> as isize));
    }
}

// TODO: is this really ok?
fn update_geometry() -> Result<(i32, i32, i32, i32), &'static str> {
    use std::mem::zeroed;
    use windows::{
        w,
        Win32::{
            Foundation::{FALSE, RECT},
            UI::WindowsAndMessaging::{
                FindWindowW, GetSystemMetrics, IsWindowVisible, SystemParametersInfoW,
                SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN,
                SPI_GETWORKAREA, SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS,
            },
        },
    };
    let (mut sx, mut sy, mut sw, mut sh) = (0, 0, 0, 0);
    let hwnd = unsafe { FindWindowW(w!("Shell_TrayWnd"), None) };
    let is_visible: bool = unsafe { IsWindowVisible(hwnd) }.into();
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
        sx = wa.left;
        sy = wa.top;
        sw = wa.right - wa.left;
        sh = wa.bottom - wa.top;
    } else {
        sx = unsafe { GetSystemMetrics(SM_XVIRTUALSCREEN) };
        sy = unsafe { GetSystemMetrics(SM_YVIRTUALSCREEN) };
        sw = unsafe { GetSystemMetrics(SM_CXVIRTUALSCREEN) };
        sh = unsafe { GetSystemMetrics(SM_CYVIRTUALSCREEN) };
    }
    Ok((sx, sy, sw, sh))
}

fn main() -> Result<(), Error> {
    let mut clients: Vec<HWND> = vec![];
    let work_area_bounds = update_geometry().unwrap();
    println!("{work_area_bounds:?}");
    enum_windows(&mut clients);
    for client in clients {
        println!("{client:?}");
    }
    Ok(())
}
