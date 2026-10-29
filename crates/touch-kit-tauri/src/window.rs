//! A panel window that is never activated: touching it, or it opening, doesn't take focus from the
//! app the user is typing in, and it stays out of the taskbar and Alt-Tab.

use tauri::{Manager, Runtime, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

/// A borderless, hidden, unfocusable window builder. Position and size it on the panel's monitor
/// after `build()`, then [`show`] it; tao's own `show()` activates the window on Windows.
pub fn builder<'a, R: Runtime, M: Manager<R>>(manager: &'a M, label: &str, url: WebviewUrl) -> WebviewWindowBuilder<'a, R, M> {
    WebviewWindowBuilder::new(manager, label, url)
        .visible(false)
        .focused(false)
        .focusable(false)
        .decorations(false)
        .resizable(false)
        .skip_taskbar(true)
        .shadow(false)
}

/// Shows `win` without activating it. On Windows it also gets `WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW`
/// and loses `WS_EX_APPWINDOW`, so clicks and touches never activate it later either.
#[cfg(windows)]
pub fn show<R: Runtime>(win: &WebviewWindow<R>) -> tauri::Result<()> {
    use windows::Win32::UI::WindowsAndMessaging::{
        GetWindowLongPtrW, SetWindowLongPtrW, SetWindowPos, ShowWindow, GWL_EXSTYLE, SWP_FRAMECHANGED, SWP_NOACTIVATE,
        SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER, SW_SHOWNOACTIVATE, WS_EX_APPWINDOW, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
    };
    let hwnd = win.hwnd()?;
    // SAFETY: plain Win32 calls on a window handle owned by this process.
    unsafe {
        let ex = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        let ex = (ex | (WS_EX_NOACTIVATE.0 | WS_EX_TOOLWINDOW.0) as isize) & !(WS_EX_APPWINDOW.0 as isize);
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, ex);
        let _ = SetWindowPos(hwnd, None, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED);
        let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
    }
    Ok(())
}

#[cfg(not(windows))]
pub fn show<R: Runtime>(win: &WebviewWindow<R>) -> tauri::Result<()> {
    win.show()
}
