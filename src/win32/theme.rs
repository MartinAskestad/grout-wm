use windows::{
    w,
    Win32::{
        Foundation::{ERROR_FILE_NOT_FOUND, ERROR_SUCCESS},
        System::Registry::{RegGetValueW, HKEY_CURRENT_USER, REG_VALUE_TYPE, RRF_RT_REG_DWORD},
    },
};

pub fn is_light_theme() -> bool {
    let mut buffer: [u8; 4] = [0; 4];
    let mut size: u32 = (buffer.len()).try_into().unwrap();
    let mut kind: REG_VALUE_TYPE = Default::default();
    let res = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            w!("Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize"),
            w!("AppsUseLightTheme"),
            RRF_RT_REG_DWORD,
            Some(&mut kind),
            Some(buffer.as_mut_ptr() as *mut _),
            Some(&mut size),
        )
    };
    if res != ERROR_SUCCESS {
        if res == ERROR_FILE_NOT_FOUND {
            return true;
        }
    }
    i32::from_le_bytes(buffer) == 1
}
