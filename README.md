# neMAX — десктопный клиент для мессенджера MAX

Лёгкая оболочка для `web.max.ru` на Rust + WebView. Фильтрует запросы, блокирует всю телеметрию и посторонние домены через allowlist.

## Особенности

- **Allowlist-фильтрация.** Все сетевые запросы WebView проходят только на разрешённые домены (`data/allowlist.txt`). Всё остальное блокируется на уровне fetch/XHR/sendBeacon.
- **Системный трей.** Сворачивание в трей: двойной левый клик по иконке — показать/скрыть окно, правый клик — меню. Закрытие на «×» прячет окно в трей (настраивается).
- **Один экземпляр.** Повторный запуск не открывает новое окно, а разворачивает уже работающее (через Windows API `CreateMutexW` + `FindWindowW`).
- **Автозапуск (Windows).** Приложение может прописываться в реестр (`HKCU\...\Run`) и стартовать свёрнутым в трей при входе в систему.
- **Подмена User-Agent.** Встроенный генератор случайных Chrome-юзер-агентов.
- **Замена звуков уведомлений.** Локальные `.mp3` из `assets/` подменяют стандартные звуки MAX.
- **Отключение анимаций.** Тяжёлые glow-анимации скрываются через CSS.
- **Кроссплатформенность.** Windows (основной) и Linux (через GTK/WebKit).

## Быстрый старт (Windows)

Скачай `nemax-windows.zip` из [Releases](https://github.com/tsrman/nemax-desktop-win/releases), распакуй в любую папку и запусти `nemax.exe`.

Для работы требуется **WebView2 Runtime** — он уже встроен в Windows 10 (21H2+) и Windows 11.

## Настройки

Все настройки через меню **Меню → Настройки**:

| Настройка | Описание |
|-----------|----------|
| User-Agent | Генерация нового случайного UA |
| Service Workers | По умолчанию отключены для приватности |
| Закрывать в трей | «×» прячет окно в трей вместо выхода |
| Запускать свёрнутым | При старте окно не показывается, только иконка в трее |
| Автозапуск (Windows) | Прописывает `nemax.exe --minimized` в реестр |
| DevTools | Инструменты разработчика WebView (только debug-сборка) |

## Allowlist

Файл `data/allowlist.txt` содержит список разрешённых доменов (по одному шаблону на строку). WebView будет загружать ресурсы только с этих доменов. При отсутствии файла используется fallback `max.ru`.

Пример:
```
max.ru
web.max.ru
okcdn.ru
```

## Сборка из исходников

Требуется [Rust](https://rustup.rs/).

### Windows
```bash
git clone https://github.com/tsrman/nemax-desktop-win.git
cd nemax-desktop-win
cargo build --release
```

### Linux
Предварительно установите зависимости WebKit:
```bash
sudo apt install libwebkit2gtk-4.1-dev build-essential libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev
```
```bash
git clone https://github.com/tsrman/nemax-desktop-win.git
cd nemax-desktop-win
cargo build --release
```

Готовый бинарник: `target/release/nemax` (Linux) или `target/release/nemax.exe` (Windows).

## Структура проекта

```
├── assets/             # Иконки, звуки
├── data/
│   └── allowlist.txt   # Разрешённые домены
├── src/
│   ├── main.rs         # Точка входа, окно, WebView, IPC
│   ├── settings.rs     # Настройки (JSON)
│   ├── tray.rs         # Системный трей
│   ├── autostart.rs    # Автозапуск (Windows Registry)
│   ├── single_instance.rs  # Защита от повторного запуска
│   ├── ua_gen.rs       # Генератор User-Agent
│   ├── filter.js       # JS-фильтр, внедряемый в WebView
│   └── settings.html   # Интерфейс настроек
├── storage/            # Настройки и данные (создаётся при запуске)
└── build.rs            # Ресурсы Windows, авто-копирование WebView2Loader.dll
```

## Лицензия и использование

Исходный код открыт для аудита. Программа делает ровно то, что заявлено: фильтрует запросы и не трогает пользовательские данные.

Пожалуйста, не клонируйте репозиторий только ради смены донат-ссылки. Если есть идеи по улучшению — Issues и Pull Requests приветствуются.
