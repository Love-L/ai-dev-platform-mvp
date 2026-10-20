use tauri::{
    AppHandle, Manager,
    menu::{Menu, MenuItem},
    tray::{TrayIconBuilder, MouseButton, MouseButtonState, TrayIconEvent},
};
use tracing::info;

/// 系统托盘图标名称（供托盘状态更新接口使用）
#[allow(dead_code)]
const TRAY_ICON_ID: &str = "ace-tray";

/// 创建系统托盘
pub fn create_tray(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    // 创建菜单项
    let show_item = MenuItem::with_id(app, "show", "显示窗口", true, None::<&str>)?;
    let hide_item = MenuItem::with_id(app, "hide", "隐藏窗口", true, None::<&str>)?;
    let separator1 = tauri::menu::PredefinedMenuItem::separator(app)?;
    let settings_item = MenuItem::with_id(app, "settings", "设置", true, None::<&str>)?;
    let separator2 = tauri::menu::PredefinedMenuItem::separator(app)?;
    let quit_item = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;

    // 创建菜单
    let menu = Menu::with_items(
        app,
        &[&show_item, &hide_item, &separator1, &settings_item, &separator2, &quit_item],
    )?;

    // 创建托盘图标
    let _tray = TrayIconBuilder::new()
        .icon(app.default_window_icon().unwrap().clone())
        .menu(&menu)
        .tooltip("ACE - AI Collaboration Engine")
        .on_menu_event(move |app, event| {
            match event.id.as_ref() {
                "show" => {
                    info!("托盘菜单: 显示窗口");
                    if let Some(window) = app.get_webview_window("main") {
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                }
                "hide" => {
                    info!("托盘菜单: 隐藏窗口");
                    if let Some(window) = app.get_webview_window("main") {
                        let _ = window.hide();
                    }
                }
                "settings" => {
                    info!("托盘菜单: 打开设置");
                    if let Some(window) = app.get_webview_window("main") {
                        let _ = window.show();
                        let _ = window.set_focus();
                        // 导航到设置页面
                        let _ = window.eval("window.location.hash = '#/settings'");
                    }
                }
                "quit" => {
                    info!("托盘菜单: 退出应用");
                    app.exit(0);
                }
                _ => {}
            }
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle();
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
        })
        .build(app)?;

    info!("系统托盘创建成功");
    Ok(())
}

/// 更新托盘提示文本（为托盘状态展示预留）
#[allow(dead_code)]
pub fn update_tray_tooltip(app: &AppHandle, tooltip: &str) {
    if let Some(tray) = app.tray_by_id(TRAY_ICON_ID) {
        let _ = tray.set_tooltip(Some(tooltip));
    }
}
