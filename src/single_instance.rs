/// Проверка единственного экземпляра. Windows — через мьютекс, Linux — через Unix socket.

#[cfg(target_os = "windows")]
mod imp {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use winapi::shared::minwindef::TRUE;
    use winapi::um::handleapi::CloseHandle;
    use winapi::um::synchapi::CreateMutexW;
    use winapi::um::winuser::{
        FindWindowW, IsIconic, SetForegroundWindow, ShowWindow, SW_RESTORE, SW_SHOWMAXIMIZED,
    };

    const MUTEX_NAME: &str = "neMAX_SingleInstance_Mutex";

    fn to_wide(s: &str) -> Vec<u16> {
        OsStr::new(s).encode_wide().chain(std::iter::once(0)).collect()
    }

    pub fn ensure_single_instance() -> bool {
        let name = to_wide(MUTEX_NAME);
        unsafe {
            let mutex = CreateMutexW(std::ptr::null_mut(), TRUE, name.as_ptr());
            if mutex.is_null() { return true; }
            let already_exists = winapi::um::errhandlingapi::GetLastError()
                == winapi::shared::winerror::ERROR_ALREADY_EXISTS;
            if already_exists {
                CloseHandle(mutex);
                let title = to_wide("neMAX");
                let hwnd = FindWindowW(std::ptr::null_mut(), title.as_ptr());
                if !hwnd.is_null() {
                    if IsIconic(hwnd) != 0 { ShowWindow(hwnd, SW_RESTORE); }
                    ShowWindow(hwnd, SW_SHOWMAXIMIZED);
                    SetForegroundWindow(hwnd);
                }
                false
            } else {
                let _ = mutex;
                true
            }
        }
    }
}

#[cfg(target_os = "linux")]
mod imp {
    use std::io::Read;
    use std::os::unix::net::{UnixListener, UnixStream};

    const SOCKET_PATH: &str = "/tmp/nemax_single_instance.sock";

    pub fn ensure_single_instance(proxy: tao::event_loop::EventLoopProxy<String>) -> bool {
        // Удаляем старый сокет если остался после краша
        let _ = std::fs::remove_file(SOCKET_PATH);

        match UnixListener::bind(SOCKET_PATH) {
            Ok(listener) => {
                // Первый экземпляр — слушаем сигналы
                std::thread::spawn(move || {
                    for stream in listener.incoming() {
                        if let Ok(mut conn) = stream {
                            let mut buf = [0u8; 4];
                            if conn.read(&mut buf).is_ok() {
                                let _ = proxy.send_event("tray:toggle".into());
                            }
                        }
                    }
                });
                true
            }
            Err(_) => {
                // Сокет занят — шлём сигнал существующему
                if let Ok(mut conn) = UnixStream::connect(SOCKET_PATH) {
                    use std::io::Write;
                    let _ = conn.write_all(b"show");
                }
                false
            }
        }
    }
}

#[cfg(not(any(target_os = "windows", target_os = "linux")))]
mod imp {
    pub fn ensure_single_instance() -> bool { true }
}

#[cfg(target_os = "windows")]
pub use imp::ensure_single_instance;

#[cfg(target_os = "linux")]
pub use imp::ensure_single_instance;
