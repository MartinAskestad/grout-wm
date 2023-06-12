#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::io::Error;
use std::panic::AssertUnwindSafe;
use windows::Win32::Foundation::{BOOL, HWND, LPARAM, LRESULT, WPARAM};

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

fn manage(hwnd: HWND, clients: &mut Vec<HWND>) -> Option<HWND> {
    if any!(clients, hwnd) {
        None
    } else {
        clients.push(hwnd);
        Some(hwnd)
    }
}

fn get_client(hwnd: HWND, clients: &mut Vec<HWND>) -> Option<HWND> {
    if any!(clients, hwnd) {
        Some(hwnd)
    } else {
        None
    }
}

fn unmanage(hwnd: HWND, clients: &mut Vec<HWND>) {
    if any!(clients, hwnd) {
        clients.retain(|h| *h != hwnd);
    }
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

fn subdivide(bounds: (i32, i32, i32, i32), vertical: bool) -> Vec<(i32, i32, i32, i32)> {
    let (bx, by, bw, bh) = bounds;
    if vertical {
        vec![(bx, by, bw / 2, bh), (bx + bw / 2, by, bw / 2, bh)]
    } else {
        vec![(bx, by, bw, bh / 2), (bx, by + bh / 2, bw, bh / 2)]
    }
}

fn spiral_subdivide(bounds: (i32, i32, i32, i32), n: usize) -> Vec<(i32, i32, i32, i32)> {
    let mut divisions = vec![bounds];
    for i in 1..n {
        let d = divisions.pop().unwrap();
        let new_d = subdivide(d, i % 2 != 0);
        divisions.extend(new_d);
    }
    divisions
}

fn arrange(clients: &Vec<HWND>) {
    use windows::Win32::UI::WindowsAndMessaging::{SetWindowPos, HWND_TOP, SWP_NOACTIVATE, IsIconic, IsWindowVisible};
    let bounds = update_geometry().unwrap();
    let mut visible_clients: Vec<HWND> = vec![];
    for c in clients {
        let min: bool = unsafe { IsIconic(c.clone()).into() };
        let vis: bool = unsafe { IsWindowVisible(c.clone()).into()  };
        if !min && vis {
            visible_clients.push(c.clone());
        }
    }
    let n = visible_clients.len();
    let ds = spiral_subdivide(bounds, n);
    for (c, d) in visible_clients.iter().zip(ds.iter()) {
        unsafe { SetWindowPos(c.clone(), HWND_TOP, d.0, d.1, d.2, d.3, SWP_NOACTIVATE); }
    }
}

static mut SHELL_HOOK_ID: u32 = 0;

unsafe extern "system" fn wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    use windows::Win32::UI::WindowsAndMessaging::{
        DefWindowProcW, GetWindowLongPtrW, PostQuitMessage, SetWindowLongPtrW, CREATESTRUCTA,
        GWLP_USERDATA, HSHELL_WINDOWACTIVATED, HSHELL_WINDOWCREATED, HSHELL_WINDOWDESTROYED,
        WM_CREATE, WM_DESTROY, WM_DISPLAYCHANGE,WM_ACTIVATE,
    };
    if msg == WM_CREATE {
        let create_struct = lparam.0 as *const CREATESTRUCTA;
        let clients = (*create_struct).lpCreateParams as *mut Vec<HWND>;
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, clients as _);
    }
    let clients = (GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut Vec<HWND>).as_mut();
    match msg {
        WM_ACTIVATE => {
            if let Some(clients) = clients {
                if is_manageable(hwnd, clients) {
                    manage(hwnd, clients);
                }
                arrange(clients);
            }
        },
        WM_DESTROY => {
            PostQuitMessage(0);
        }
        WM_DISPLAYCHANGE => {
            let _ = update_geometry(); // TODO: make this work
        }
        _ => {
            if let Some(clients) = clients {
                if msg == SHELL_HOOK_ID {
                    let hwnd = HWND(lparam.0);
                    let c = get_client(hwnd, clients);
                    match wparam.0 as u32 & 0x7FFF {
                        HSHELL_WINDOWCREATED => {
                            if c.is_none() && is_manageable(hwnd, clients) {
                                manage(hwnd, clients);
                                arrange(clients);
                            }
                        }
                        HSHELL_WINDOWDESTROYED => {
                            if c.is_some() {
                                unmanage(hwnd, clients);
                                arrange(clients);
                            }
                        }
                        HSHELL_WINDOWACTIVATED => {
                            println!("window activated {lparam:?}");
                        }
                        _ => {}
                    }
                }
            }
            return DefWindowProcW(hwnd, msg, wparam, lparam);
        }
    }
    LRESULT(0)
}

fn main() -> Result<(), Error> {
    use std::os::raw::c_void;
    use windows::{
        w,
        Win32::{
            Foundation::FALSE,
            System::LibraryLoader::GetModuleHandleA,
            UI::WindowsAndMessaging::{
                CreateWindowExW, DeregisterShellHookWindow, DispatchMessageW, GetMessageW,
                LoadCursorW, PostMessageW, RegisterClassW, RegisterShellHookWindow,
                RegisterWindowMessageW, ShowWindow, TranslateMessage, CW_USEDEFAULT, IDC_ARROW,
                MSG, SW_SHOWMINNOACTIVE, WINDOW_EX_STYLE, WM_USER, WNDCLASSW, WS_OVERLAPPEDWINDOW,
            },
        },
    };
    let instance = unsafe { GetModuleHandleA(None)? };
    assert_ne!(instance.0, 0, "Could not get instance");
    let window_class = w!("grout-wm.window");
    let wc = WNDCLASSW {
        hCursor: unsafe { LoadCursorW(None, IDC_ARROW).unwrap() },
        hInstance: instance,
        lpszClassName: window_class,
        lpfnWndProc: Some(wnd_proc),
        ..Default::default()
    };
    let reg = unsafe { RegisterClassW(&wc) };
    assert_ne!(reg, 0, "Could not register class");
    let mut clients: Vec<HWND> = vec![];
    enum_windows(&mut clients);
    let clients_ptr: *mut c_void = &mut clients as *mut _ as *mut c_void;
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
            instance,
            Some(clients_ptr),
        )
    };
    assert_ne!(hwnd.0, 0, "Could not create window");
    unsafe {
        ShowWindow(hwnd, SW_SHOWMINNOACTIVE);
    }
    let res = unsafe { RegisterShellHookWindow(hwnd) };
    assert_ne!(res, FALSE, "Could not register shell hook window");
    unsafe {
        SHELL_HOOK_ID = RegisterWindowMessageW(w!("SHELLHOOK"));
    }
    let mut message = MSG::default();
    unsafe {
        while GetMessageW(&mut message, HWND(0), 0, 0).into() {
            TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }
    unsafe {
        DeregisterShellHookWindow(hwnd);
    }
    Ok(())
}
