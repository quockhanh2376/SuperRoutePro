#[cfg(target_os = "windows")]
use windows_sys::Win32::Foundation::CloseHandle;
#[cfg(target_os = "windows")]
use windows_sys::Win32::Security::{GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY};
#[cfg(target_os = "windows")]
use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

#[cfg(target_os = "windows")]
pub fn is_process_elevated() -> Result<bool, String> {
    let mut token_handle = std::ptr::null_mut();
    let open_ok = unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token_handle) };
    if open_ok == 0 || token_handle.is_null() {
        return Err("Unable to query the current process token.".to_string());
    }

    let mut elevation = TOKEN_ELEVATION { TokenIsElevated: 0 };
    let mut return_length = 0u32;
    let info_ok = unsafe {
        GetTokenInformation(
            token_handle,
            TokenElevation,
            &mut elevation as *mut TOKEN_ELEVATION as *mut _,
            std::mem::size_of::<TOKEN_ELEVATION>() as u32,
            &mut return_length,
        )
    };

    unsafe {
        let _ = CloseHandle(token_handle);
    }

    if info_ok == 0 {
        return Err("Unable to read the current process elevation state.".to_string());
    }

    Ok(elevation.TokenIsElevated != 0)
}

#[cfg(not(target_os = "windows"))]
pub fn is_process_elevated() -> Result<bool, String> {
    Ok(false)
}
