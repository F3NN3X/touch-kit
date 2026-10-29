//! Creates a panel window on its monitor without ever activating it.

use crate::config::PanelEntry;
use tauri::{AppHandle, PhysicalPosition, PhysicalSize, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

/// The parts of a monitor that matter for matching, in physical px.
#[derive(Clone, Debug, PartialEq)]
pub struct MonitorInfo {
    pub name: Option<String>,
    pub position: [i32; 2],
    pub size: [u32; 2],
}

/// The panel's monitor: the one its `monitor` names, else the first whose size equals the panel's.
/// Monitors at a position in `taken` (other panels') are skipped either way.
pub fn pick_monitor<'a>(monitors: &'a [MonitorInfo], entry: &PanelEntry, taken: &[[i32; 2]]) -> Option<&'a MonitorInfo> {
    let mut free = monitors.iter().filter(|m| !taken.contains(&m.position));
    if let Some(m) = &entry.monitor {
        return free.find(|mon| {
            m.name.as_ref().is_none_or(|n| mon.name.as_ref() == Some(n)) && m.position.is_none_or(|p| mon.position == p)
        });
    }
    let size = [entry.panel.width_px, entry.panel.height_px];
    free.find(|mon| mon.size == size)
}

pub fn monitors(app: &AppHandle) -> anyhow::Result<Vec<MonitorInfo>> {
    Ok(app
        .available_monitors()?
        .iter()
        .map(|m| MonitorInfo {
            name: m.name().cloned(),
            position: [m.position().x, m.position().y],
            size: [m.size().width, m.size().height],
        })
        .collect())
}

/// `route`: the page's hash (`""` for the panel UI, `"#pair/<name>"` for pairing).
pub fn create(app: &AppHandle, label: &str, mon: &MonitorInfo, dev_mode: bool, route: &str) -> anyhow::Result<WebviewWindow> {
    let win = WebviewWindowBuilder::new(app, label, WebviewUrl::App(format!("index.html{route}").into()))
        .title("Touch panel")
        .visible(false)
        .focused(false)
        .focusable(false)
        .decorations(false)
        .resizable(false)
        .skip_taskbar(true)
        .shadow(false)
        .build()?;
    win.set_position(PhysicalPosition::new(mon.position[0], mon.position[1]))?;
    win.set_size(PhysicalSize::new(mon.size[0], mon.size[1]))?;
    if dev_mode {
        win.open_devtools();
    } else {
        // Touch arrives from touch-core; the real mouse must never land on this window.
        win.set_ignore_cursor_events(true)?;
    }
    show(&win)?;
    Ok(win)
}

/// A testing mode panel: an ordinary window (title bar, movable, takes the mouse) at `spot`.
pub fn create_virtual(app: &AppHandle, label: &str, title: &str, spot: &MonitorInfo) -> anyhow::Result<WebviewWindow> {
    let win = WebviewWindowBuilder::new(app, label, WebviewUrl::App("index.html".into()))
        .title(title)
        .visible(false)
        .focused(false)
        .resizable(false)
        .build()?;
    win.set_position(PhysicalPosition::new(spot.position[0], spot.position[1]))?;
    win.set_size(PhysicalSize::new(spot.size[0], spot.size[1]))?;
    win.show()?;
    Ok(win)
}

#[cfg(windows)]
fn show(win: &WebviewWindow) -> anyhow::Result<()> {
    crate::win::show_without_activating(win.hwnd()?);
    Ok(())
}

#[cfg(not(windows))]
fn show(win: &WebviewWindow) -> anyhow::Result<()> {
    Ok(win.show()?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::MonitorMatch;

    fn mon(name: &str, position: [i32; 2], size: [u32; 2]) -> MonitorInfo {
        MonitorInfo { name: Some(name.into()), position, size }
    }

    fn monitors() -> Vec<MonitorInfo> {
        // This machine, from the Gate 2 dev run.
        vec![
            mon(r"\\.\DISPLAY2", [0, 0], [3840, 2160]),
            mon(r"\\.\DISPLAY3", [596, -1440], [2560, 1440]),
            mon(r"\\.\DISPLAY1", [3840, -1262], [1440, 3440]),
            mon(r"\\.\DISPLAY4", [3833, 2178], [1920, 550]),
        ]
    }

    #[test]
    fn default_matches_by_panel_size() {
        let ms = monitors();
        let m = pick_monitor(&ms, &PanelEntry::default(), &[]).unwrap();
        assert_eq!(m.name.as_deref(), Some(r"\\.\DISPLAY4"));
    }

    #[test]
    fn override_by_name_or_position_wins_over_size() {
        let ms = monitors();
        let by_name = PanelEntry {
            monitor: Some(MonitorMatch { name: Some(r"\\.\DISPLAY3".into()), position: None }),
            ..Default::default()
        };
        assert_eq!(pick_monitor(&ms, &by_name, &[]).unwrap().position, [596, -1440]);
        let by_pos = PanelEntry { monitor: Some(MonitorMatch { name: None, position: Some([3840, -1262]) }), ..Default::default() };
        assert_eq!(pick_monitor(&ms, &by_pos, &[]).unwrap().name.as_deref(), Some(r"\\.\DISPLAY1"));
    }

    #[test]
    fn no_match_is_none_not_a_fallback() {
        let ms = monitors();
        let mut entry = PanelEntry::default();
        entry.panel.height_px = 480;
        assert!(pick_monitor(&ms, &entry, &[]).is_none(), "must never fall back to a main screen");
        entry.monitor = Some(MonitorMatch { name: Some("nope".into()), position: None });
        assert!(pick_monitor(&ms, &entry, &[]).is_none());
    }

    #[test]
    fn a_monitor_another_panel_has_is_skipped() {
        let mut ms = monitors();
        ms.push(mon(r"\\.\DISPLAY5", [5753, 2178], [1920, 550]));
        let second = pick_monitor(&ms, &PanelEntry::default(), &[[3833, 2178]]).unwrap();
        assert_eq!(second.name.as_deref(), Some(r"\\.\DISPLAY5"));
        assert!(pick_monitor(&ms, &PanelEntry::default(), &[[3833, 2178], [5753, 2178]]).is_none());
    }
}
