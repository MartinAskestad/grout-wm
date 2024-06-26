use log::error;
use std::{
    ffi::c_void,
    mem::{size_of, zeroed},
    path::PathBuf,
};

use windows::{
    core::{w, PCWSTR},
    Win32::{
        Foundation::{
            CloseHandle, GetLastError, ERROR_ALREADY_EXISTS, FALSE, HANDLE, HMODULE, HWND, LPARAM,
            LRESULT, MAX_PATH, POINT, RECT, TRUE, WPARAM,
        },
        Graphics::Gdi::PtInRect,
        System::{
            LibraryLoader::GetModuleHandleA,
            ProcessStatus::{
                EnumProcessModules, GetModuleBaseNameW, GetModuleInformation, MODULEINFO,
            },
            Threading::{
                CreateMutexW, OpenProcess, ReleaseMutex, PROCESS_QUERY_INFORMATION, PROCESS_VM_READ,
            },
        },
        UI::{
            Accessibility::{SetWinEventHook, HWINEVENTHOOK, WINEVENTPROC},
            Shell::{FOLDERID_LocalAppData, SHGetKnownFolderPath, KF_FLAG_DEFAULT},
            WindowsAndMessaging::{
                BeginDeferWindowPos, DefWindowProcW, DeferWindowPos, EndDeferWindowPos,
                EnumWindows, FindWindowW, GetClassNameW, GetCursorPos, GetSystemMetrics, GetWindow,
                GetWindowLongPtrW, GetWindowTextLengthW, GetWindowTextW, GetWindowThreadProcessId,
                IsIconic, IsWindowVisible, LoadIconW, PostMessageW, PostQuitMessage,
                RegisterClassW, RegisterShellHookWindow, RegisterWindowMessageW, SetWindowLongPtrW,
                ShowWindow, SystemParametersInfoW, GET_WINDOW_CMD, GWL_EXSTYLE, GWL_STYLE, HDWP,
                HICON, HWND_TOP, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN,
                SM_YVIRTUALSCREEN, SPI_GETWORKAREA, SWP_NOACTIVATE, SW_SHOWMINNOACTIVE,
                SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS, WINDOW_LONG_PTR_INDEX, WINEVENT_OUTOFCONTEXT,
                WNDCLASSW, WNDENUMPROC,
            },
        },
    },
};

use grout_wm::Result;

pub(crate) mod com;
pub(crate) mod dwm;
pub(crate) mod taskbar;
pub(crate) mod theme;
pub(crate) mod virtualdesktop;

pub fn begin_defer_window_pos(num_windows: usize) -> Result<HDWP> {
    Ok(unsafe { BeginDeferWindowPos(num_windows as i32)? })
}

pub fn defer_window_pos(hdwp: HDWP, hwnd: HWND, rect: RECT) -> Result<HDWP> {
    let margin = dwm::get_window_extended_frame_bounds(hwnd); // Should be: (left: 7, top: 0, right: -7, bottom: -7)
    let res = unsafe {
        DeferWindowPos(
            hdwp,
            hwnd,
            HWND_TOP,
            rect.left - margin.left,
            rect.top - margin.top,
            (rect.right - rect.left) + margin.left * 2,
            (rect.bottom - rect.top) - margin.bottom,
            SWP_NOACTIVATE,
        )?
    };
    Ok(res)
}

pub fn end_defer_window_pos(hdwp: HDWP) {
    let res = unsafe { EndDeferWindowPos(hdwp) };
    if res.is_err() {
        error!("EndDeferWindowPos failed: {:?}", res);
    }
}

pub fn is_iconic(hwnd: HWND) -> bool {
    unsafe { IsIconic(hwnd).into() }
}

pub fn get_window_long_ptr(hwnd: HWND, nindex: WINDOW_LONG_PTR_INDEX) -> isize {
    unsafe { GetWindowLongPtrW(hwnd, nindex) }
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

pub fn get_working_area() -> Result<RECT> {
    let hwnd = unsafe { FindWindowW(w!("Shell_TrayWnd"), None) };
    let is_visible = is_window_visible(hwnd);
    if hwnd.0 != 0 && is_visible {
        let mut wa: RECT = unsafe { zeroed() };
        let res = unsafe {
            SystemParametersInfoW(
                SPI_GETWORKAREA,
                0,
                Some(&mut wa as *mut RECT as *mut c_void),
                SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
            )
        };
        if res.is_err() {
            return Err("".into());
        }
        Ok(wa)
    } else {
        Ok(RECT {
            left: unsafe { GetSystemMetrics(SM_XVIRTUALSCREEN) },
            top: unsafe { GetSystemMetrics(SM_YVIRTUALSCREEN) },
            right: unsafe { GetSystemMetrics(SM_CXVIRTUALSCREEN) },
            bottom: unsafe { GetSystemMetrics(SM_CYVIRTUALSCREEN) },
        })
    }
}

pub fn get_module_handle() -> windows::core::Result<HMODULE> {
    unsafe { GetModuleHandleA(None) }
}

pub fn register_class(wc: *const WNDCLASSW) -> u16 {
    unsafe { RegisterClassW(wc) }
}

pub fn register_shell_hook_window(hwnd: HWND) -> bool {
    unsafe { RegisterShellHookWindow(hwnd).into() }
}

pub fn register_window_messagew(s: PCWSTR) -> u32 {
    unsafe { RegisterWindowMessageW(s) }
}

pub fn set_win_event_hook(eventmin: u32, eventmax: u32, wndproc: WINEVENTPROC) -> HWINEVENTHOOK {
    unsafe {
        SetWinEventHook(
            eventmin,
            eventmax,
            None,
            wndproc,
            0,
            0,
            WINEVENT_OUTOFCONTEXT,
        )
    }
}

pub fn show_window(hwnd: HWND) -> bool {
    unsafe { ShowWindow(hwnd, SW_SHOWMINNOACTIVE).into() }
}

pub fn enum_windows(cb: WNDENUMPROC, param: LPARAM) -> bool {
    unsafe { EnumWindows(cb, param).is_ok() }
}

pub fn get_window_text(hwnd: HWND) -> String {
    let mut buf: [u16; 512] = [0; 512];
    let len = unsafe { GetWindowTextW(hwnd, &mut buf) };
    String::from_utf16_lossy(&buf[..len as usize])
}

pub fn get_window_classname(hwnd: HWND) -> String {
    let mut buf: [u16; 512] = [0; 512];
    let len = unsafe { GetClassNameW(hwnd, &mut buf) };
    String::from_utf16_lossy(&buf[..len as usize])
}

pub fn get_exe_filename(hwnd: HWND) -> Option<String> {
    let mut process_id: u32 = 0;
    unsafe {
        GetWindowThreadProcessId(hwnd, Some(&mut process_id));
        let process_handle_res = OpenProcess(
            PROCESS_QUERY_INFORMATION | PROCESS_VM_READ,
            FALSE,
            process_id,
        );
        if let Ok(process_handle) = process_handle_res {
            let mut module_handles: [HMODULE; 1024] = [HMODULE(0); 1024];
            let mut module_handles_size = 0;
            if EnumProcessModules(
                process_handle,
                module_handles.as_mut_ptr(),
                (module_handles.len() * size_of::<HMODULE>()) as u32,
                &mut module_handles_size,
            )
            .is_err()
            {
                return None;
            }
            let module_handle = module_handles[0];
            let mut module_info: MODULEINFO = zeroed();
            if GetModuleInformation(
                process_handle,
                module_handle,
                &mut module_info,
                size_of::<MODULEINFO>().try_into().unwrap(),
            )
            .is_err()
            {
                return None;
            }
            let mut module_base_name: [u16; MAX_PATH as usize] = [0; MAX_PATH as usize];
            let base_name_length =
                GetModuleBaseNameW(process_handle, module_handle, &mut module_base_name);
            if base_name_length == 0 {
                return None;
            }
            let res = CloseHandle(process_handle);
            if res.is_err() {
                error!("Failed to close process handle: {}", res.unwrap_err());
            }
            Some(String::from_utf16_lossy(
                &module_base_name[..base_name_length as usize],
            ))
        } else {
            None
        }
    }
}

pub fn def_window_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
}

pub fn post_quit_message(msg: i32) {
    unsafe { PostQuitMessage(msg) }
}

pub fn post_message(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> Result<()> {
    unsafe { PostMessageW(hwnd, msg, wparam, lparam).unwrap() };
    Ok(())
}

pub fn get_mutex() -> Result<HANDLE> {
    let mutex_name = w!("wm-mutex");
    let mutex_handle = unsafe { CreateMutexW(None, TRUE, mutex_name) };
    if unsafe { GetLastError() } == ERROR_ALREADY_EXISTS {
        Err("Mutex already exists".into())
    } else {
        Ok(mutex_handle.unwrap())
    }
}

pub fn release_mutex(handle: HANDLE) {
    let mutex_res = unsafe { ReleaseMutex(handle) };
    if mutex_res.is_err() {
        error!("ReleaseMutex failed: {:?}", mutex_res);
    }
    let handle_res = unsafe { CloseHandle(handle) };
    if handle_res.is_err() {
        error!("CloseHandle failed: {:?}", handle_res);
    }
}

pub fn get_window(hwnd: HWND, ucmd: GET_WINDOW_CMD) -> HWND {
    unsafe { GetWindow(hwnd, ucmd) }
}

pub fn set_window_long_ptr(hwnd: HWND, nindex: WINDOW_LONG_PTR_INDEX, dwnewlong: isize) -> isize {
    unsafe { SetWindowLongPtrW(hwnd, nindex, dwnewlong) }
}

pub fn get_local_appdata_path() -> Result<PathBuf> {
    let wide_path = unsafe {
        SHGetKnownFolderPath(&FOLDERID_LocalAppData, KF_FLAG_DEFAULT, HANDLE::default())?
    };
    let path_str = unsafe { wide_path.to_string().unwrap() };
    let path = PathBuf::from(path_str);
    Ok(path)
}

pub fn get_cursor_pos() -> POINT {
    let mut p: POINT = unsafe { zeroed() };
    let res = unsafe { GetCursorPos(&mut p) };
    if res.is_err() {
        error!("GetCursorPos failed: {:?}", res);
    }
    p
}

pub fn point_in_rect(lprc: RECT, pt: POINT) -> bool {
    unsafe { PtInRect(&lprc, pt).into() }
}

pub fn load_icon(hinstance: HMODULE, lpiconname: PCWSTR) -> windows::core::Result<HICON> {
    unsafe { LoadIconW(hinstance, lpiconname) }
}
