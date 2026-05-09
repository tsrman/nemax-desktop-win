use tao::event_loop::EventLoopProxy;
use tray_icon::{
    menu::{Menu, MenuEvent, MenuEventReceiver, MenuItem},
    Icon, MouseButton, TrayIcon, TrayIconBuilder, TrayIconEvent,
};

pub const TRAY_ID_SHOW_HIDE: &str = "tray_show_hide";
pub const TRAY_ID_EXIT: &str = "tray_exit";

/// Состояние иконки в системном трее.
pub struct TrayState {
    pub tray: TrayIcon,
    pub show_hide_item: MenuItem,
    pub menu_events: MenuEventReceiver,
}

impl TrayState {
    /// Создаёт иконку в трее с меню и настраивает обработку событий.
    /// Двойной левый клик отправляет `"tray:toggle"` через `proxy`.
    pub fn new(
        icon_rgba: Vec<u8>,
        icon_w: u32,
        icon_h: u32,
        visible: bool,
        proxy: EventLoopProxy<String>,
    ) -> Self {
        let icon = Icon::from_rgba(icon_rgba, icon_w, icon_h)
            .expect("Failed to create tray icon from RGBA");

        let menu = Menu::new();
        let label = if visible { "Скрыть" } else { "Показать" };
        let show_hide = MenuItem::with_id(TRAY_ID_SHOW_HIDE, label, true, None);
        let exit = MenuItem::with_id(TRAY_ID_EXIT, "Выход", true, None);
        menu.append(&show_hide).expect("Failed to append Show/Hide item");
        menu.append(&exit).expect("Failed to append Exit item");

        // set_event_handler: двойной левый → toggle
        TrayIconEvent::set_event_handler(Some(move |event: TrayIconEvent| {
            if let TrayIconEvent::DoubleClick { button: MouseButton::Left, .. } = event {
                let _ = proxy.send_event("tray:toggle".into());
            }
        }));

        let tray = TrayIconBuilder::new()
            .with_icon(icon)
            .with_menu(Box::new(menu))
            .with_menu_on_left_click(false)
            .with_tooltip("neMAX")
            .build()
            .expect("Failed to build tray icon");

        let menu_events = MenuEvent::receiver().clone();

        Self {
            tray,
            show_hide_item: show_hide,
            menu_events,
        }
    }
}
