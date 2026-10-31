use windows::Win32::Foundation::{ERROR_FILE_NOT_FOUND, ERROR_SUCCESS, WIN32_ERROR};
use windows::Win32::System::Registry::{
    HKEY, HKEY_CURRENT_USER, REG_SZ, RegCloseKey, RegCreateKeyW, RegDeleteValueW, RegSetValueExW,
};
use windows::core::w;

const VALUE_NAME: windows::core::PCWSTR = w!("Wiggler");
const RUN_KEY: windows::core::PCWSTR = w!("Software\\Microsoft\\Windows\\CurrentVersion\\Run");

pub fn set_enabled(enabled: bool) -> Result<(), String> {
    if enabled { enable() } else { disable() }
}

fn enable() -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|err| err.to_string())?;
    let command = format!("\"{}\" --autostart", exe.display());
    let key = open_run_key()?;
    let data = wide_bytes(&command);
    let status = unsafe { RegSetValueExW(key, VALUE_NAME, None, REG_SZ, Some(&data)) };
    close_key(key);
    check(status, "could not write the sign-in entry")
}

fn disable() -> Result<(), String> {
    let key = open_run_key()?;
    let status = unsafe { RegDeleteValueW(key, VALUE_NAME) };
    close_key(key);
    if status == ERROR_FILE_NOT_FOUND {
        return Ok(());
    }
    check(status, "could not remove the sign-in entry")
}

fn open_run_key() -> Result<HKEY, String> {
    let mut key = HKEY::default();
    let status = unsafe { RegCreateKeyW(HKEY_CURRENT_USER, RUN_KEY, &mut key) };
    check(status, "could not open the sign-in registry key")?;
    Ok(key)
}

fn close_key(key: HKEY) {
    unsafe {
        let _ = RegCloseKey(key);
    }
}

fn check(status: WIN32_ERROR, message: &str) -> Result<(), String> {
    if status == ERROR_SUCCESS {
        Ok(())
    } else {
        Err(format!("{message} ({})", status.0))
    }
}

fn wide_bytes(value: &str) -> Vec<u8> {
    value
        .encode_utf16()
        .chain(std::iter::once(0))
        .flat_map(u16::to_le_bytes)
        .collect()
}
