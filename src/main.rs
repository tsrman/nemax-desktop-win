#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod settings;
mod ua_gen;

use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::sync::{Arc, Mutex};

use muda::{Menu, MenuItem, PredefinedMenuItem, Submenu};
use tao::{
    dpi::LogicalSize,
    event::{Event, StartCause, WindowEvent},
    event_loop::{ControlFlow, EventLoopBuilder},
    window::{Icon, WindowBuilder},
};
use tracing::{error, info, warn};
use tracing_subscriber::EnvFilter;
use wry::WebViewBuilder;

use settings::Settings;

// ---------------------------------------------------------------------------
// Конфигурация
// ---------------------------------------------------------------------------

const APP_TITLE:    &str = "neMAX";
const APP_ENDPOINT: &str = "https://web.max.ru";
const DONATE_URL:   &str = "https://pay.cloudtips.ru/p/fe3fd493"; 

const DIR_ASSETS:  &str = "assets";
const DIR_DATA:    &str = "data";
const DIR_STORAGE: &str = "storage";

const FILE_ICON:      &str = "assets/icon.png";
const FILE_BLOCKLIST: &str = "data/blocklist.txt";

const DEFAULT_BLOCKLIST: &[&str] = &[
    "analytics", "yandex", "google-analytics", "facebook", "ads", "gtag",
    "track", "mixpanel", "apptracer", "perf/upload", "crashtoken", "sdkversion",
    "amplitude", "hotjar", "doubleclick", "googletagmanager", "segment",
    "matomo", "adsystem", "sdk-api", "perf/", "crash",
];

const FILTER_SCRIPT_TEMPLATE: &str = include_str!("filter.js");
const SETTINGS_HTML:          &str = include_str!("settings.html");

// ID пунктов меню для muda
const MENU_SETTINGS: &str = "menu_settings";
const MENU_DONATE:   &str = "menu_donate";
const MENU_ABOUT:    &str = "menu_about";

// ---------------------------------------------------------------------------
// Команды IPC от окна настроек
// ---------------------------------------------------------------------------

enum SettingsCmd {
    NewUA,
    SetSW(bool),
    OpenDevTools,
    Close,
}

impl SettingsCmd {
    fn parse(msg: &str) -> Option<Self> {
        match msg {
            "settings:new_ua"   => Some(Self::NewUA),
            "settings:devtools" => Some(Self::OpenDevTools),
            "settings:close"    => Some(Self::Close),
            s if s.starts_with("settings:set_sw:") => {
                let val = s.trim_start_matches("settings:set_sw:") == "true";
                Some(Self::SetSW(val))
            }
            _ => None,
        }
    }
}

// ---------------------------------------------------------------------------
// Утилиты
// ---------------------------------------------------------------------------

fn init_directories() {
    for dir in[DIR_ASSETS, DIR_DATA, DIR_STORAGE] {
        if !Path::new(dir).exists() {
            if let Err(e) = fs::create_dir_all(dir) {
                error!("Cannot create directory '{}': {}", dir, e);
            }
        }
    }
}

fn load_icon(path: impl AsRef<Path>) -> Option<Icon> {
    let path = path.as_ref();
    info!("Loading icon: {}", path.display());
    match image::open(path) {
        Ok(img) => {
            let (w, h) = (img.width(), img.height());
            Icon::from_rgba(img.into_rgba8().into_raw(), w, h)
                .map_err(|e| error!("Icon conversion failed: {}", e))
                .ok()
        }
        Err(e) => {
            error!("Icon load failed: {}", e);
            None
        }
    }
}

fn load_blocklist(path: impl AsRef<Path>) -> Vec<String> {
    let path = path.as_ref();
    if let Ok(file) = File::open(path) {
        let list: Vec<String> = BufReader::new(file)
            .lines()
            .filter_map(|l| l.ok())
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty() && !s.starts_with('#'))
            .collect();
        if !list.is_empty() {
            info!("Loaded {} blocklist entries", list.len());
            return list;
        }
        warn!("Blocklist is empty, using defaults");
    } else {
        info!("Blocklist not found, using defaults");
    }
    DEFAULT_BLOCKLIST.iter().map(|s| s.to_string()).collect()
}

use base64::{Engine as _, engine::general_purpose}; // Добавьте в начало файла

fn build_filter_script(blocklist: &[String], allow_sw: bool) -> String {
    let sigs_json = serde_json::to_string(blocklist).unwrap_or_else(|_| "[]".to_string());
    
    // Вспомогательная функция для чтения в Base64
    let load_sound = |path: &str| -> String {
        if let Ok(bytes) = std::fs::read(path) {
            general_purpose::STANDARD.encode(bytes)
        } else {
            "".to_string()
        }
    };

    let notif_b64 = load_sound("assets/notification.mp3");
    let ring_b64  = load_sound("assets/ringtone.mp3");
    let recon_b64 = load_sound("assets/reconnect.mp3");

    FILTER_SCRIPT_TEMPLATE
        .replace("__SIGNATURES__", &sigs_json)
        .replace("__ALLOW_SW__", if allow_sw { "true" } else { "false" })
        .replace("__NOTIF_B64__", &notif_b64)
        .replace("__RING_B64__", &ring_b64)
        .replace("__RECON_B64__", &recon_b64)
}

// ---------------------------------------------------------------------------
// Точка входа
// ---------------------------------------------------------------------------

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    init_directories();
    info!("Application starting");

    let settings = Arc::new(Mutex::new(Settings::load()));

    let (user_agent, allow_sw) = {
        let s = settings.lock().unwrap();
        (s.user_agent.clone(), s.allow_service_workers)
    };

    // Создаем EventLoop с поддержкой кастомных событий (чтобы сообщения от веб-страниц мгновенно будили программу)
    let event_loop = EventLoopBuilder::<String>::with_user_event().build();
    let proxy = event_loop.create_proxy();

    // --- Меню "Файл" через muda ---
    let menu_bar = Menu::new();
    let file_menu = Submenu::new("Меню", true);

    let _ = file_menu.append_items(&[
        &MenuItem::with_id(MENU_SETTINGS, "Настройки", true, None),
        &PredefinedMenuItem::separator(),
        &MenuItem::with_id(MENU_DONATE, "Донат ♥", true, None),
        &MenuItem::with_id(MENU_ABOUT, "О приложении", true, None),
        &PredefinedMenuItem::separator(),
        &PredefinedMenuItem::quit(None),
    ]);

    let _ = menu_bar.append(&file_menu);

    // --- Главное окно ---
    let mut window_builder = WindowBuilder::new()
        .with_title(APP_TITLE)
        .with_visible(false)
        .with_inner_size(LogicalSize::new(1200.0_f64, 800.0_f64))
        .with_resizable(true);

    if let Some(icon) = load_icon(FILE_ICON) {
        window_builder = window_builder.with_window_icon(Some(icon));
    }

    let main_window    = window_builder.build(&event_loop).expect("Window creation failed");
    let main_window_id = main_window.id();

    #[cfg(target_os = "windows")]
    {
        use tao::platform::windows::WindowExtWindows;
        #[allow(unused_unsafe)]
        unsafe { let _ = menu_bar.init_for_hwnd(main_window.hwnd() as _); }
    }
    #[cfg(target_os = "macos")]
    {
        let _ = menu_bar.init_for_nsapp();
    }
    #[cfg(target_os = "linux")]
    {
        use tao::platform::unix::WindowExtUnix;
        let _ = menu_bar.init_for_gtk_window(main_window.gtk_window(), None::<&muda::gtk::Container>);
    }

    // --- Главный WebView ---
    let blocklist     = load_blocklist(FILE_BLOCKLIST);
    let filter_script = build_filter_script(&blocklist, allow_sw);

    let main_proxy = proxy.clone();
    let mut main_wv_builder = WebViewBuilder::new()
        .with_url(APP_ENDPOINT)
        .with_user_agent(&user_agent)
        .with_initialization_script(&filter_script)
        .with_ipc_handler(move |request: wry::http::Request<String>| {
            let _ = main_proxy.send_event(request.body().clone());
        })

        .with_custom_protocol("nemax".into(), move |_id, request| {
            let path = request.uri().path();
            // Если запрашивается файл из папки assets
            if path.starts_with("/assets/") {
                let file_name = &path[8..]; // обрезаем "/assets/"
                let asset_path = std::path::Path::new("assets").join(file_name);
                
                if let Ok(content) = std::fs::read(asset_path) {
                    return wry::http::Response::builder()
                        .header("Content-Type", "audio/mpeg")
                        .header("Access-Control-Allow-Origin", "*") // Разрешаем CORS
                        .body(content.into())
                        .unwrap();
                }
            }
            wry::http::Response::builder().status(404).body(vec![].into()).unwrap()
        });

    #[cfg(debug_assertions)]
    {
        main_wv_builder = main_wv_builder.with_devtools(true);
    }

    let main_webview = main_wv_builder.with_background_color((18, 18, 18, 255)).build(&main_window).expect("WebView creation failed");

    info!("Initialization complete");

    let mut settings_win: Option<(tao::window::Window, wry::WebView)> = None;
    let mut donate_win:   Option<(tao::window::Window, wry::WebView)> = None;
    
    let menu_channel = muda::MenuEvent::receiver();

    event_loop.run(move |event, event_loop_target, control_flow| {
        *control_flow = ControlFlow::Wait;

        // --- Разбираем события от меню muda ---
        while let Ok(menu_event) = menu_channel.try_recv() {
            if menu_event.id == MENU_SETTINGS {
                if settings_win.is_none() {
                    let s = settings.lock().unwrap();
                    let settings_json = serde_json::to_string(&*s)
                        .unwrap_or_else(|_| "{}".to_string());
                    drop(s);

                    // Улучшенный скрипт инициализации: точно сработает
                    let init_script = format!(
                        r#"
                        window.__SETTINGS_JSON__ = {};
                        var __settingsInterval = setInterval(function() {{
                            if (typeof window.__applySettings === 'function') {{
                                window.__applySettings(window.__SETTINGS_JSON__);
                                clearInterval(__settingsInterval);
                            }}
                        }}, 50);

                        // Подстраховка: перестаем пытаться через 3 секунды, чтобы не крутить цикл вечно
                        setTimeout(function() {{ clearInterval(__settingsInterval); }}, 3000);
                        "#,
                        settings_json
                    );

                    let win = WindowBuilder::new()
                        .with_title("Настройки — neMAX")
                        .with_inner_size(LogicalSize::new(520.0_f64, 500.0_f64))
                        .with_resizable(false)
                        .build(event_loop_target)
                        .expect("Settings window failed");

                    let settings_proxy = proxy.clone();
                    let wv = WebViewBuilder::new()
                        .with_html(SETTINGS_HTML)
                        .with_initialization_script(&init_script)
                        .with_ipc_handler(move |req: wry::http::Request<String>| {
                            let _ = settings_proxy.send_event(req.body().clone());
                        })
                        .build(&win)
                        .expect("Settings WebView failed");

                    settings_win = Some((win, wv));
                    info!("Settings window opened");
                } else if let Some((ref win, _)) = settings_win {
                    win.set_focus();
                }
            } else if menu_event.id == MENU_DONATE {
                // Донат теперь открывается в отдельном окне приложения
                if donate_win.is_none() {
                    let win = WindowBuilder::new()
                        .with_title("Донат ♥ — neMAX")
                        .with_inner_size(LogicalSize::new(800.0_f64, 600.0_f64))
                        .with_resizable(true)
                        .build(event_loop_target)
                        .expect("Donate window failed");

                    let wv = WebViewBuilder::new()
                        .with_url(DONATE_URL)
                        .build(&win)
                        .expect("Donate WebView failed");

                    donate_win = Some((win, wv));
                    info!("Donate window opened");
                } else if let Some((ref win, _)) = donate_win {
                    win.set_focus();
                }
            } else if menu_event.id == MENU_ABOUT {
                let script = format!(
                    "alert('neMAX v{}\\n\\nЛёгкий и безопасный клиент для max.ru\\n\\nДанное ПО является независимой разработкой и не связано с ООО «ВК» (ИНН 7743001840), ООО «Коммуникационная платформа» (ИНН 9714058267), ООО «МАХ» (ИНН 9714058267), ООО «Мэйл.ру Цифровые Технологии» (ИНН 7714415613) и структурами VK (Mail.ru Group).\\n\\nДанные передаются на сервер в исходном виде. Разработчик не дополняет и не модифицирует запросы (за исключением блокировки исходящей аналитики/статистики).\\n\\nУсловия использования (EULA):\\n- tos.nemax-mod.ru\\n- https://telegra.ph/LICENZIONNOE-SOGLASHENIE-KONECHNOGO-POLZOVATELYA-EULA-12-10\\n\\n© 2026 neMAX');",
                    env!("CARGO_PKG_VERSION")
                );
                if let Err(e) = main_webview.evaluate_script(&script) {
                    error!("About dialog failed: {}", e);
                }
            }
        }

        match event {
            // --- Обработка пользовательских сообщений (мгновенная реакция) ---
            Event::UserEvent(msg) => {
                if msg.starts_with("settings:") {
                    info!("[Settings IPC] {}", msg);
                    match SettingsCmd::parse(&msg) {
                        Some(SettingsCmd::NewUA) => {
                            let new_ua = settings.lock().unwrap().randomize_ua();
                            info!("New UA: {}", new_ua);
                            if let Some((_, ref wv)) = settings_win {
                                let script = format!(
                                    "if(window.__updateUA) window.__updateUA({});",
                                    serde_json::to_string(&new_ua).unwrap_or_else(|_| "\"\"".into())
                                );
                                let _ = wv.evaluate_script(&script);
                            }
                        }
                        Some(SettingsCmd::SetSW(val)) => {
                            settings.lock().unwrap().set_service_workers(val);
                        }
                        Some(SettingsCmd::OpenDevTools) => {
                            #[cfg(debug_assertions)]
                            main_webview.open_devtools();
                            
                            #[cfg(not(debug_assertions))]
                            if let Some((_, ref wv)) = settings_win {
                                let _ = wv.evaluate_script(
                                    "alert('DevTools доступны только в debug-сборке (cargo run).');"
                                );
                            }
                        }
                        Some(SettingsCmd::Close) => {
                            settings_win = None;
                        }
                        None => {}
                    }
                } else if msg == "filter:ready" {
                    main_window.set_visible(true);
                    main_window.set_focus(); // Сразу даем фокус
                    info!("Window shown after content ready");
                } else if msg.starts_with("filter:") {
                    let log_msg = &msg[7..];
                    if log_msg.contains("blocked") {
                        warn!("[FILTER] {}", log_msg);
                    } else {
                        info!("[IPC] {}", log_msg);
                    }
                }
            }

            Event::NewEvents(StartCause::Init) => {
                info!("Window ready");
            }

            Event::WindowEvent { event: WindowEvent::CloseRequested, window_id, .. } => {
                if window_id == main_window_id {
                    info!("Main window closed, exiting");
                    *control_flow = ControlFlow::Exit;
                } else if let Some((ref win, _)) = settings_win {
                    if win.id() == window_id {
                        settings_win = None;
                        info!("Settings window closed");
                    }
                }
                if let Some((ref win, _)) = donate_win {
                    if win.id() == window_id {
                        donate_win = None;
                        info!("Donate window closed");
                    }
                }
            }
            _ => {}
        }
    });
}