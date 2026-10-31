use windows::Win32::Foundation::{GetLastError, HANDLE};
use windows::Win32::System::StationsAndDesktops::{
    CloseDesktop, DESKTOP_CONTROL_FLAGS, DESKTOP_READOBJECTS, GetUserObjectInformationW,
    OpenInputDesktop, UOI_NAME,
};
use windows::Win32::System::SystemInformation::GetTickCount;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetLastInputInfo, INPUT, INPUT_MOUSE, LASTINPUTINFO, MOUSEEVENTF_MOVE, MOUSEINPUT, SendInput,
};
use windows::Win32::UI::WindowsAndMessaging::{
    SPI_GETSCREENSAVERRUNNING, SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS, SystemParametersInfoW,
};

pub fn idle_ms() -> u64 {
    unsafe {
        let mut info = LASTINPUTINFO {
            cbSize: size_of::<LASTINPUTINFO>() as u32,
            dwTime: 0,
        };
        if !GetLastInputInfo(&mut info).as_bool() {
            return 0;
        }
        u64::from(GetTickCount().wrapping_sub(info.dwTime))
    }
}

pub fn nudge_mouse() -> Result<(), String> {
    let inputs = [mouse_move(1, 0), mouse_move(-1, 0)];
    let sent = unsafe { SendInput(&inputs, size_of::<INPUT>() as i32) };
    if sent != inputs.len() as u32 {
        let error = unsafe { GetLastError() };
        return Err(format!(
            "SendInput failed (sent={sent}, error={}, size={})",
            error.0,
            size_of::<INPUT>()
        ));
    }
    Ok(())
}

fn mouse_move(dx: i32, dy: i32) -> INPUT {
    let mut input = INPUT::default();
    input.r#type = INPUT_MOUSE;
    input.Anonymous.mi = MOUSEINPUT {
        dx,
        dy,
        mouseData: 0,
        dwFlags: MOUSEEVENTF_MOVE,
        time: 0,
        dwExtraInfo: 0,
    };
    input
}

pub fn nudge_once() -> Result<(), Box<dyn std::error::Error>> {
    attach_console();
    std::thread::sleep(std::time::Duration::from_millis(400));
    let before = idle_ms();
    nudge_mouse()?;
    std::thread::sleep(std::time::Duration::from_millis(50));
    let after = idle_ms();
    println!("SendInput size={} idle_ms before={before} after={after}", size_of::<INPUT>());
    if after > 200 {
        return Err("idle clock did not reset".into());
    }
    println!("nudge ok");
    Ok(())
}

pub fn screen_is_locked_or_saver() -> bool {
    screensaver_running() || !input_desktop_is_default()
}

fn screensaver_running() -> bool {
    let mut running: i32 = 0;
    unsafe {
        SystemParametersInfoW(
            SPI_GETSCREENSAVERRUNNING,
            0,
            Some((&raw mut running).cast()),
            SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
        )
    }
    .is_ok()
        && running != 0
}

fn input_desktop_is_default() -> bool {
    let Ok(desk) = (unsafe { OpenInputDesktop(DESKTOP_CONTROL_FLAGS(0), false, DESKTOP_READOBJECTS) })
    else {
        return false;
    };
    let mut name = [0u16; 256];
    let ok = unsafe {
        GetUserObjectInformationW(
            HANDLE(desk.0),
            UOI_NAME,
            Some(name.as_mut_ptr().cast()),
            (name.len() * 2) as u32,
            None,
        )
    };
    let _ = unsafe { CloseDesktop(desk) };
    if ok.is_err() {
        return false;
    }
    let end = name.iter().position(|&c| c == 0).unwrap_or(name.len());
    String::from_utf16_lossy(&name[..end]).eq_ignore_ascii_case("Default")
}

#[cfg(test)]
mod tests {
    use super::screen_is_locked_or_saver;

    #[test]
    fn unlocked_session_is_not_blocked() {
        assert!(
            !screen_is_locked_or_saver(),
            "this session is unlocked, so lock/screensaver detection should be clear"
        );
    }
}

fn attach_console() {
    use windows::Win32::System::Console::{ATTACH_PARENT_PROCESS, AttachConsole};
    unsafe {
        let _ = AttachConsole(ATTACH_PARENT_PROCESS);
    }
}
