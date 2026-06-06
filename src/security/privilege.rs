use anyhow::{Context, Result};
use tracing::info;

/// Check if the current process is running with elevated privileges.
pub fn is_elevated() -> bool {
    #[cfg(windows)]
    {
        is_elevated_windows()
    }
    #[cfg(unix)]
    {
        unsafe { libc::geteuid() == 0 }
    }
    #[cfg(not(any(windows, unix)))]
    {
        false
    }
}

#[cfg(windows)]
fn is_elevated_windows() -> bool {
    use winapi::um::processthreadsapi::{GetCurrentProcess, OpenProcessToken};
    use winapi::um::securitybaseapi::GetTokenInformation;
    use winapi::um::winnt::{TokenElevation, HANDLE, TOKEN_QUERY};

    unsafe {
        let mut token: HANDLE = std::ptr::null_mut();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) == 0 {
            return false;
        }

        let mut elevation: u32 = 0;
        let mut size: u32 = 0;
        let result = GetTokenInformation(
            token,
            TokenElevation,
            &mut elevation as *mut _ as *mut winapi::ctypes::c_void,
            std::mem::size_of::<u32>() as u32,
            &mut size,
        );

        result != 0 && elevation != 0
    }
}

/// Self-elevate on Windows via UAC. Saves session state before relaunch.
#[cfg(windows)]
pub fn self_elevate_windows(args: &[&str]) -> Result<()> {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use winapi::um::shellapi::ShellExecuteExW;
    use winapi::um::shellapi::SHELLEXECUTEINFOW;
    use winapi::um::winuser::SW_SHOW;

    let exe = std::env::current_exe().context("Failed to get current exe path")?;
    let exe_wide: Vec<u16> = OsStr::new(exe.as_os_str())
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    let params = args.join(" ");
    let params_wide: Vec<u16> = OsStr::new(&params)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    let runas_verb: Vec<u16> = std::ffi::OsStr::new("runas")
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    unsafe {
        let mut sei: SHELLEXECUTEINFOW = std::mem::zeroed();
        sei.cbSize = std::mem::size_of::<SHELLEXECUTEINFOW>() as u32;
        sei.fMask = 0;
        sei.hwnd = std::ptr::null_mut();
        sei.lpVerb = runas_verb.as_ptr();
        sei.lpFile = exe_wide.as_ptr();
        sei.lpParameters = params_wide.as_ptr();
        sei.lpDirectory = std::ptr::null();
        sei.nShow = SW_SHOW;

        if ShellExecuteExW(&mut sei) == 0 {
            anyhow::bail!("UAC elevation failed (error {})", std::io::Error::last_os_error());
        }
    }

    info!("Self-elevated via UAC. Parent process exiting.");
    Ok(())
}

/// Request sudo elevation on Unix by asking user for password.
#[cfg(unix)]
pub fn request_sudo_elevation(args: &[&str]) -> Result<()> {
    let exe = std::env::current_exe().context("Failed to get current exe path")?;
    let status = std::process::Command::new("sudo")
        .args(["-E", "--preserve-env=VO1D_PATH,VO1D_SESSION"])
        .arg(&exe)
        .args(args)
        .status()
        .context("Failed to execute sudo")?;

    if !status.success() {
        anyhow::bail!("sudo elevation failed with exit code: {:?}", status.code());
    }
    Ok(())
}

/// Cache sudo credentials (YOLO mode on Linux).
#[cfg(unix)]
pub fn cache_sudo_credentials() -> Result<()> {
    let status = std::process::Command::new("sudo")
        .args(["-v"])
        .status()
        .context("Failed to cache sudo credentials")?;

    if !status.success() {
        anyhow::bail!("Failed to cache sudo credentials");
    }
    info!("Sudo credentials cached (valid ~5 minutes)");
    Ok(())
}
