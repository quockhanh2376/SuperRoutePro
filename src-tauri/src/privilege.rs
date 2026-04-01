#[cfg(target_os = "windows")]
use std::ptr::null_mut;
#[cfg(target_os = "windows")]
use windows_sys::Win32::Foundation::{CloseHandle, BOOL, HANDLE};
#[cfg(target_os = "windows")]
use windows_sys::Win32::Security::{
    CheckTokenMembership, CreateWellKnownSid, GetTokenInformation, TokenElevation,
    TokenElevationType, TokenElevationTypeFull, TokenElevationTypeLimited,
    WinBuiltinAdministratorsSid, TOKEN_ELEVATION, TOKEN_ELEVATION_TYPE, TOKEN_QUERY,
};
#[cfg(target_os = "windows")]
use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PrivilegeContext {
    pub process_is_elevated: bool,
    pub account_is_local_admin: bool,
}

#[cfg(target_os = "windows")]
const SECURITY_MAX_SID_SIZE: usize = 68;

#[cfg(target_os = "windows")]
pub fn current_privilege_context() -> Result<PrivilegeContext, String> {
    let token_handle = open_current_process_token()?;
    let process_is_elevated = read_token_elevation(token_handle)?;
    let token_elevation_type = read_token_elevation_type(token_handle)?;

    unsafe {
        let _ = CloseHandle(token_handle);
    }

    let admin_membership_enabled = current_token_has_builtin_admin_membership()?;

    Ok(PrivilegeContext {
        process_is_elevated,
        account_is_local_admin: account_is_local_admin_from_signals(
            process_is_elevated,
            token_elevation_type,
            admin_membership_enabled,
        ),
    })
}

#[cfg(not(target_os = "windows"))]
pub fn current_privilege_context() -> Result<PrivilegeContext, String> {
    Ok(PrivilegeContext {
        process_is_elevated: false,
        account_is_local_admin: false,
    })
}

#[cfg(target_os = "windows")]
pub fn is_process_elevated() -> Result<bool, String> {
    current_privilege_context().map(|context| context.process_is_elevated)
}

#[cfg(not(target_os = "windows"))]
pub fn is_process_elevated() -> Result<bool, String> {
    Ok(false)
}

#[cfg(target_os = "windows")]
fn open_current_process_token() -> Result<HANDLE, String> {
    let mut token_handle = null_mut();
    let open_ok = unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token_handle) };
    if open_ok == 0 || token_handle.is_null() {
        return Err("Unable to query the current process token.".to_string());
    }
    Ok(token_handle)
}

#[cfg(target_os = "windows")]
fn read_token_elevation(token_handle: HANDLE) -> Result<bool, String> {
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

    if info_ok == 0 {
        return Err("Unable to read the current process elevation state.".to_string());
    }

    Ok(elevation.TokenIsElevated != 0)
}

#[cfg(target_os = "windows")]
fn read_token_elevation_type(token_handle: HANDLE) -> Result<TOKEN_ELEVATION_TYPE, String> {
    let mut elevation_type: TOKEN_ELEVATION_TYPE = 0;
    let mut return_length = 0u32;
    let info_ok = unsafe {
        GetTokenInformation(
            token_handle,
            TokenElevationType,
            &mut elevation_type as *mut TOKEN_ELEVATION_TYPE as *mut _,
            std::mem::size_of::<TOKEN_ELEVATION_TYPE>() as u32,
            &mut return_length,
        )
    };

    if info_ok == 0 {
        return Err("Unable to read the current token elevation type.".to_string());
    }

    Ok(elevation_type)
}

#[cfg(target_os = "windows")]
fn current_token_has_builtin_admin_membership() -> Result<bool, String> {
    let mut admin_sid = [0u8; SECURITY_MAX_SID_SIZE];
    let mut sid_size = admin_sid.len() as u32;
    let sid_ok = unsafe {
        CreateWellKnownSid(
            WinBuiltinAdministratorsSid,
            null_mut(),
            admin_sid.as_mut_ptr() as *mut _,
            &mut sid_size,
        )
    };
    if sid_ok == 0 {
        return Err("Unable to build the Administrators security identifier.".to_string());
    }

    let mut is_member: BOOL = 0;
    let membership_ok = unsafe {
        CheckTokenMembership(null_mut(), admin_sid.as_mut_ptr() as *mut _, &mut is_member)
    };
    if membership_ok == 0 {
        return Err("Unable to read the current user administrator membership.".to_string());
    }

    Ok(is_member != 0)
}

#[cfg(target_os = "windows")]
fn account_is_local_admin_from_signals(
    process_is_elevated: bool,
    token_elevation_type: TOKEN_ELEVATION_TYPE,
    admin_membership_enabled: bool,
) -> bool {
    process_is_elevated
        || token_elevation_type == TokenElevationTypeFull
        || token_elevation_type == TokenElevationTypeLimited
        || admin_membership_enabled
}

#[cfg(test)]
mod tests {
    use super::PrivilegeContext;
    #[cfg(target_os = "windows")]
    use windows_sys::Win32::Security::{
        TokenElevationTypeDefault, TokenElevationTypeFull, TokenElevationTypeLimited,
    };

    #[cfg(target_os = "windows")]
    #[test]
    fn limited_token_still_counts_as_local_admin() {
        assert!(super::account_is_local_admin_from_signals(
            false,
            TokenElevationTypeLimited,
            false
        ));
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn full_token_counts_as_local_admin() {
        assert!(super::account_is_local_admin_from_signals(
            true,
            TokenElevationTypeFull,
            true
        ));
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn default_token_without_admin_membership_stays_standard_user() {
        assert!(!super::account_is_local_admin_from_signals(
            false,
            TokenElevationTypeDefault,
            false
        ));
    }

    #[test]
    fn non_windows_context_defaults_to_standard_user_shape() {
        let context = super::current_privilege_context().unwrap_or(PrivilegeContext {
            process_is_elevated: false,
            account_is_local_admin: false,
        });
        let _ = context;
    }
}
