use windows::Win32::Foundation::CloseHandle;
use windows::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS,
};
use windows::Win32::System::Threading::{
    GetCurrentProcessId, OpenProcess, TerminateProcess, WaitForSingleObject, PROCESS_SYNCHRONIZE,
    PROCESS_TERMINATE,
};

const WAIT_MS: u32 = 3000;

pub fn running_instances() {
    let _ = kill_others();
}

fn kill_others() -> windows::core::Result<()> {
    let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) }?;
    let self_pid = unsafe { GetCurrentProcessId() };
    let mut entry = PROCESSENTRY32W {
        dwSize: size_of::<PROCESSENTRY32W>() as u32,
        ..Default::default()
    };
    let mut more = unsafe { Process32FirstW(snapshot, &mut entry) };
    while more.is_ok() {
        if exe_name(&entry.szExeFile).eq_ignore_ascii_case("wiggler.exe")
            && entry.th32ProcessID != self_pid
        {
            terminate(entry.th32ProcessID);
        }
        more = unsafe { Process32NextW(snapshot, &mut entry) };
    }
    unsafe { CloseHandle(snapshot) }
}

fn exe_name(raw: &[u16; 260]) -> String {
    let len = raw.iter().position(|&c| c == 0).unwrap_or(raw.len());
    String::from_utf16_lossy(&raw[..len])
}

fn terminate(pid: u32) {
    let Ok(handle) = (unsafe { OpenProcess(PROCESS_TERMINATE | PROCESS_SYNCHRONIZE, false, pid) })
    else {
        return;
    };
    let _ = unsafe { TerminateProcess(handle, 1) };
    unsafe {
        WaitForSingleObject(handle, WAIT_MS);
    }
    let _ = unsafe { CloseHandle(handle) };
}
