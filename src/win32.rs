use std::{
    ffi::c_void,
    mem::{size_of, zeroed},
    path::PathBuf,
};

use log::info;
use windows::{
    core::PCWSTR,
    w,
    Win32::{
        Foundation::{
            CloseHandle, GetLastError, BOOL, ERROR_ALREADY_EXISTS, FALSE, HANDLE, HMODULE, HWND,
            LPARAM, LRESULT, MAX_PATH, RECT, TRUE, WPARAM,
        },
        Graphics::Dwm::{
            DwmGetWindowAttribute, DWMWA_CLOAKED, DWM_CLOAKED_APP, DWM_CLOAKED_INHERITED,
            DWM_CLOAKED_SHELL, DWMWA_EXTENDED_FRAME_BOUNDS,
        },
        System::{
            Com::{CoCreateInstance, CoInitialize, CoUninitialize, CLSCTX_ALL},
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
            Shell::{
                FOLDERID_LocalAppData, IVirtualDesktopManager, SHGetKnownFolderPath,
                VirtualDesktopManager as VirtualDesktopManager_ID, KF_FLAG_DEFAULT,
            },
            WindowsAndMessaging::{
                DefWindowProcW, EnumWindows, FindWindowW, GetClassNameW, GetSystemMetrics,
                GetWindow, GetWindowLongPtrW, GetWindowTextLengthW, GetWindowTextW,
                GetWindowThreadProcessId, IsIconic, IsWindowVisible, PostMessageW, PostQuitMessage,
                RegisterClassW, RegisterShellHookWindow, RegisterWindowMessageW, SetWindowLongPtrW,
                SetWindowPos, ShowWindow, SystemParametersInfoW, GET_WINDOW_CMD, GWL_EXSTYLE,
                GWL_STYLE, HWND_TOP, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN,
                SM_YVIRTUALSCREEN, SPI_GETWORKAREA, SWP_NOACTIVATE, SW_SHOWMINNOACTIVE,
                SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS, WINDOW_LONG_PTR_INDEX, WINEVENT_OUTOFCONTEXT,
                WNDCLASSW, WNDENUMPROC, GetWindowRect,
            },
        },
    },
};

use grout_wm::Result;

use crate::rect::Rect;

pub struct Win32Com;

impl Win32Com {
    pub fn new() -> Result<Self> {
        info!("Initialize COM");
        unsafe {
            CoInitialize(None)?;
        }
        Ok(Win32Com)
    }
}

impl Drop for Win32Com {
    fn drop(&mut self) {
        info!("Uninitializing COM");
        unsafe {
            CoUninitialize();
        }
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

pub fn get_working_area() -> Result<Rect> {
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
        if res == FALSE {
            return Err("".into());
        }
        Ok(Rect::from (wa))
    } else {
        Ok(Rect::new(
            unsafe { GetSystemMetrics(SM_XVIRTUALSCREEN) },
            unsafe { GetSystemMetrics(SM_YVIRTUALSCREEN) },
            unsafe { GetSystemMetrics(SM_CXVIRTUALSCREEN) },
            unsafe { GetSystemMetrics(SM_CYVIRTUALSCREEN) },
        ))
    }
}

pub fn set_window_pos(hwnd: HWND, r: Rect) {
    let offset = get_window_border_offset(hwnd);
    let pos = r - offset;
    unsafe {
        SetWindowPos(hwnd, HWND_TOP, pos.left, pos.top, pos.width, pos.height, SWP_NOACTIVATE);
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
    unsafe { EnumWindows(cb, param).into() }
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
            ) == FALSE
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
            ) == FALSE
            {
                return None;
            }
            let mut module_base_name: [u16; MAX_PATH as usize] = [0; MAX_PATH as usize];
            let base_name_length =
                GetModuleBaseNameW(process_handle, module_handle, &mut module_base_name);
            if base_name_length == 0 {
                return None;
            }
            CloseHandle(process_handle);
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

pub fn post_message(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> BOOL {
    unsafe { PostMessageW(hwnd, msg, wparam, lparam) }
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
    unsafe {
        ReleaseMutex(handle);
        CloseHandle(handle);
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

pub fn get_window_border_offset(hwnd: HWND) -> RECT {
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
    let mut border: RECT = Default::default();
    border.left = frame.left - rect.left;
    border.top = frame.top - rect.top;
    border.right = frame.right - rect.right;
    border.bottom = frame.bottom - rect.bottom;
border
}

// =====
// Virtual desktop
// =====

pub struct VirtualDesktopManager(IVirtualDesktopManager);

impl VirtualDesktopManager {
    pub fn new() -> Result<Self> {
        info!("Instantiate VirtualDesktopManager");
        unsafe {
            CoInitialize(None)?;
        }
        let virtual_desktop_managr =
            unsafe { CoCreateInstance(&VirtualDesktopManager_ID, None, CLSCTX_ALL)? };
        Ok(Self(virtual_desktop_managr))
    }

    pub fn is_window_on_current_desktop(&self, hwnd: HWND) -> windows::core::Result<bool> {
        let is_on_desktop = unsafe { self.0.IsWindowOnCurrentVirtualDesktop(hwnd)? };
        Ok(is_on_desktop.as_bool())
    }
}
