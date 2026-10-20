//! 启动期致命错误的可见提示。
//!
//! 发布版应用无控制台（`windows_subsystem = "windows"`），裸 `panic`/`expect`
//! 只表现为"点了图标没反应"。本模块用系统对话框把错误与处置建议讲清楚，
//! 再退出进程，替代不可见的闪退。

/// 弹出致命错误对话框并退出进程。
///
/// Windows 使用 `MessageBoxW`；其它平台输出到 stderr 后退出。
pub fn show_fatal_and_exit(title: &str, message: &str) -> ! {
    tracing::error!("{} — {}", title, message);

    #[cfg(windows)]
    unsafe {
        message_box_w(title, message);
    }

    #[cfg(not(windows))]
    {
        eprintln!("FATAL {}: {}", title, message);
    }

    std::process::exit(1);
}

#[cfg(windows)]
#[link(name = "user32")]
extern "system" {
    fn MessageBoxW(
        hWnd: isize,
        lp_text: *const u16,
        lp_caption: *const u16,
        u_type: u32,
    ) -> i32;
}

/// Windows 系统错误框：确定按钮 + 错误图标 + 前台置顶。
#[cfg(windows)]
unsafe fn message_box_w(title: &str, message: &str) {
    const MB_OK: u32 = 0x0;
    const MB_ICONERROR: u32 = 0x10;
    const MB_SETFOREGROUND: u32 = 0x1_0000;

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    let text = wide(message);
    let caption = wide(title);
    MessageBoxW(
        0,
        text.as_ptr(),
        caption.as_ptr(),
        MB_OK | MB_ICONERROR | MB_SETFOREGROUND,
    );
}
