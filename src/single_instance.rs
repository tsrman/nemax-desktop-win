/// Проверка единственного экземпляра через Windows API.
/// Если другой экземпляр уже запущен — разворачивает его окно и возвращает `false`.
/// Иначе возвращает `true` (можно продолжать запуск).

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

    /// Уникальное имя мьютекса
    const MUTEX_NAME: &str = "neMAX_SingleInstance_Mutex";

    fn to_wide(s: &str) -> Vec<u16> {
        OsStr::new(s).encode_wide().chain(std::iter::once(0)).collect()
    }

    pub fn ensure_single_instance() -> bool {
        let name = to_wide(MUTEX_NAME);

        unsafe {
            let mutex = CreateMutexW(std::ptr::null_mut(), TRUE, name.as_ptr());

            if mutex.is_null() {
                return true; // на всякий случай разрешаем запуск
            }

            // Если мьютекс уже существует — ERROR_ALREADY_EXISTS
            let already_exists =
                winapi::um::errhandlingapi::GetLastError() == winapi::shared::winerror::ERROR_ALREADY_EXISTS;

            if already_exists {
                // Закрываем хендл — он нам не нужен
                CloseHandle(mutex);

                // Ищем окно по заголовку
                let title = to_wide("neMAX");
                let hwnd = FindWindowW(std::ptr::null_mut(), title.as_ptr());

                if !hwnd.is_null() {
                    // Если окно свёрнуто — восстанавливаем
                    if IsIconic(hwnd) != 0 {
                        ShowWindow(hwnd, SW_RESTORE);
                    }
                    // Разворачиваем и выносим на передний план
                    ShowWindow(hwnd, SW_SHOWMAXIMIZED);
                    SetForegroundWindow(hwnd);
                }

                false
            } else {
                // Первый экземпляр — держим хендл до завершения процесса
                let _ = mutex; // утечка намеренная, освободится при смерти процесса
                true
            }
        }
    }
}

#[cfg(not(target_os = "windows"))]
mod imp {
    pub fn ensure_single_instance() -> bool {
        // На других платформах — пока без проверки
        true
    }
}

pub use imp::ensure_single_instance;
