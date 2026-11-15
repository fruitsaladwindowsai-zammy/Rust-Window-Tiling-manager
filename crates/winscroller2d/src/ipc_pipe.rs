use crate::win32::{to_wide, WM_APP_IPC_NOTIFY};
use std::sync::mpsc;
use winscroller2d_core::{IpcRequest, IpcResponse};
use windows_sys::Win32::Foundation::{
    CloseHandle, GENERIC_READ, GENERIC_WRITE, HANDLE, INVALID_HANDLE_VALUE,
};
use windows_sys::Win32::Security::{
    InitializeSecurityDescriptor, SetSecurityDescriptorDacl, SECURITY_ATTRIBUTES,
    SECURITY_DESCRIPTOR,
};

const SECURITY_DESCRIPTOR_REVISION: u32 = 1;
use windows_sys::Win32::Storage::FileSystem::{
    CreateFileW, FlushFileBuffers, ReadFile, WriteFile, FILE_ATTRIBUTE_NORMAL, OPEN_EXISTING,
    PIPE_ACCESS_DUPLEX,
};
use windows_sys::Win32::System::Pipes::{
    ConnectNamedPipe, CreateNamedPipeW, DisconnectNamedPipe, WaitNamedPipeW, PIPE_READMODE_BYTE,
    PIPE_TYPE_BYTE, PIPE_UNLIMITED_INSTANCES, PIPE_WAIT,
};
use windows_sys::Win32::UI::WindowsAndMessaging::PostMessageW;

pub const PIPE_NAME: &str = r"\\.\pipe\winscroller2d-ipc";

pub struct PendingIpc {
    pub req: IpcRequest,
    pub reply_tx: mpsc::Sender<IpcResponse>,
}

pub fn spawn_ipc_server(
    target_hwnd: usize,
) -> mpsc::Receiver<PendingIpc> {
    let (tx, rx) = mpsc::channel::<PendingIpc>();
    std::thread::Builder::new()
        .name("winscroller2d-ipc".into())
        .spawn(move || {
            let pipe_w = to_wide(PIPE_NAME);
            loop {
                // Initialize a NULL DACL security descriptor so non-elevated user clients
                // can communicate with an elevated daemon without Access Denied (UIPI).
                let mut sd: SECURITY_DESCRIPTOR = unsafe { std::mem::zeroed() };
                unsafe {
                    InitializeSecurityDescriptor(
                        &mut sd as *mut _ as *mut _,
                        SECURITY_DESCRIPTOR_REVISION,
                    );
                    SetSecurityDescriptorDacl(&mut sd as *mut _ as *mut _, 1, std::ptr::null_mut(), 0);
                }
                let mut sa = SECURITY_ATTRIBUTES {
                    nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
                    lpSecurityDescriptor: &mut sd as *mut _ as *mut _,
                    bInheritHandle: 0,
                };

                let pipe: HANDLE = unsafe {
                    CreateNamedPipeW(
                        pipe_w.as_ptr(),
                        PIPE_ACCESS_DUPLEX,
                        PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT,
                        PIPE_UNLIMITED_INSTANCES,
                        65536,
                        65536,
                        0,
                        &mut sa,
                    )
                };
                if pipe == INVALID_HANDLE_VALUE {
                    std::thread::sleep(std::time::Duration::from_millis(200));
                    continue;
                }

                let connected = unsafe { ConnectNamedPipe(pipe, std::ptr::null_mut()) };
                if connected == 0 {
                    // ERROR_PIPE_CONNECTED (535) means client connected between Create & Connect.
                    let err = unsafe { windows_sys::Win32::Foundation::GetLastError() };
                    if err != 535 {
                        unsafe { CloseHandle(pipe) };
                        continue;
                    }
                }

                let tx_clone = tx.clone();
                let pipe_raw = pipe as usize;
                std::thread::spawn(move || {
                    handle_pipe_client(pipe_raw as HANDLE, target_hwnd, tx_clone);
                });
            }
        })
        .expect("spawn ipc thread");
    rx
}

fn handle_pipe_client(
    pipe: HANDLE,
    target_hwnd: usize,
    tx: mpsc::Sender<PendingIpc>,
) {
    let mut buf = [0u8; 65536];
    let mut read_bytes: u32 = 0;
    let ok = unsafe {
        ReadFile(
            pipe,
            buf.as_mut_ptr() as *mut _,
            buf.len() as u32,
            &mut read_bytes,
            std::ptr::null_mut(),
        )
    };

    let response = if ok != 0 && read_bytes > 0 {
        let text = String::from_utf8_lossy(&buf[..read_bytes as usize]);
        match serde_json::from_str::<IpcRequest>(text.trim()) {
            Ok(req) => {
                let (reply_tx, reply_rx) = mpsc::channel();
                if tx.send(PendingIpc { req, reply_tx }).is_ok() {
                    unsafe {
                        PostMessageW(target_hwnd as _, WM_APP_IPC_NOTIFY, 0, 0);
                    }
                    reply_rx
                        .recv_timeout(std::time::Duration::from_secs(3))
                        .unwrap_or(IpcResponse::Error {
                            message: "daemon timed out".into(),
                        })
                } else {
                    IpcResponse::Error {
                        message: "daemon shutting down".into(),
                    }
                }
            }
            Err(e) => IpcResponse::Error {
                message: format!("invalid IPC request JSON: {e}"),
            },
        }
    } else {
        IpcResponse::Error {
            message: "empty IPC request".into(),
        }
    };

    if let Ok(mut json) = serde_json::to_vec(&response) {
        json.push(b'\n');
        let mut written: u32 = 0;
        unsafe {
            WriteFile(
                pipe,
                json.as_ptr() as *const _,
                json.len() as u32,
                &mut written,
                std::ptr::null_mut(),
            );
            FlushFileBuffers(pipe);
        }
    }

    unsafe {
        DisconnectNamedPipe(pipe);
        CloseHandle(pipe);
    }
}

pub fn send_ipc_request(req: &IpcRequest) -> Result<IpcResponse, String> {
    let pipe_w = to_wide(PIPE_NAME);
    unsafe {
        WaitNamedPipeW(pipe_w.as_ptr(), 1500);
    }
    let handle = unsafe {
        CreateFileW(
            pipe_w.as_ptr(),
            GENERIC_READ | GENERIC_WRITE,
            0,
            std::ptr::null(),
            OPEN_EXISTING,
            FILE_ATTRIBUTE_NORMAL,
            std::ptr::null_mut(),
        )
    };
    if handle == INVALID_HANDLE_VALUE {
        return Err("could not connect to WinScroller 2D daemon (is winscroller2d running?)".into());
    }

    let mut payload = serde_json::to_vec(req).map_err(|e| e.to_string())?;
    payload.push(b'\n');

    let mut written: u32 = 0;
    let write_ok = unsafe {
        WriteFile(
            handle,
            payload.as_ptr() as *const _,
            payload.len() as u32,
            &mut written,
            std::ptr::null_mut(),
        )
    };
    if write_ok == 0 {
        unsafe { CloseHandle(handle) };
        return Err("failed writing IPC request".into());
    }

    let mut out = Vec::new();
    let mut chunk = [0u8; 8192];
    loop {
        let mut read_bytes: u32 = 0;
        let read_ok = unsafe {
            ReadFile(
                handle,
                chunk.as_mut_ptr() as *mut _,
                chunk.len() as u32,
                &mut read_bytes,
                std::ptr::null_mut(),
            )
        };
        if read_ok == 0 || read_bytes == 0 {
            break;
        }
        out.extend_from_slice(&chunk[..read_bytes as usize]);
        if out.ends_with(b"\n") {
            break;
        }
    }
    unsafe { CloseHandle(handle) };

    let line = String::from_utf8_lossy(&out);
    serde_json::from_str::<IpcResponse>(line.trim())
        .map_err(|e| format!("invalid response from daemon: {e}"))
}
