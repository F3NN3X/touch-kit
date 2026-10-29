//! Testing mode's virtual panels: what they play and where their windows go (config `testing`).

use crate::window::MonitorInfo;
use touch_core::Recording;

/// The captures shipped in the binary, by name; `all` plays them one after another.
pub const BUILT_IN: [&str; 5] = ["all", "tap", "swipes", "two-finger", "hold"];

const CAPTURES: [(&str, &str); 4] = [
    ("tap", include_str!("../../../../captures/tap.jsonl")),
    ("swipes", include_str!("../../../../captures/swipes.jsonl")),
    ("two-finger", include_str!("../../../../captures/two-finger.jsonl")),
    ("hold", include_str!("../../../../captures/hold.jsonl")),
];

/// Pause between two captures in `all`.
const GAP_MS: u64 = 1500;

/// The recording `replay` names. `none` is the panel's descriptor without reports: connected, idle.
pub fn recording(replay: &str) -> Result<Recording, String> {
    let built_in = |name: &str| {
        let (_, text) = CAPTURES.iter().find(|(n, _)| *n == name).expect("known capture");
        Recording::parse(text, name).map_err(|e| e.to_string())
    };
    match replay {
        "none" => Ok(Recording { reports: Vec::new(), ..built_in("tap")? }),
        "all" => {
            let mut all = Recording { reports: Vec::new(), ..built_in("tap")? };
            for (name, _) in CAPTURES {
                let offset = all.reports.last().map_or(0, |(t, _)| t + GAP_MS);
                all.reports.extend(built_in(name)?.reports.into_iter().map(|(t, r)| (t + offset, r)));
            }
            Ok(all)
        }
        name if BUILT_IN.contains(&name) => built_in(name),
        path => Recording::load(path.as_ref()).map_err(|e| e.to_string()),
    }
}

/// Window spots for `n` virtual panels on `main`: each the panel's size in CSS px (`size` × the
/// monitor's `scale`, so the UI lays out as on the real panel), stacked down from the top-left and
/// wrapping back to the top when the monitor runs out of height.
pub fn slots(main: &MonitorInfo, scale: f64, size: [u32; 2], n: usize) -> Vec<MonitorInfo> {
    const MARGIN: i32 = 40;
    let w = (f64::from(size[0]) * scale).round() as u32;
    let h = (f64::from(size[1]) * scale).round() as u32;
    let step = h as i32 + MARGIN;
    let rows = ((main.size[1] as i32 - MARGIN) / step).max(1);
    (0..n as i32)
        .map(|k| MonitorInfo {
            name: Some(format!("virtual {}", k + 1)),
            position: [main.position[0] + MARGIN * (1 + k / rows), main.position[1] + MARGIN + (k % rows) * step],
            size: [w, h],
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_replay_name_loads() {
        for name in BUILT_IN.iter().chain(&["none"]) {
            recording(name).unwrap_or_else(|e| panic!("{name}: {e}"));
        }
        let all = recording("all").unwrap();
        assert!(all.reports.windows(2).all(|w| w[0].0 <= w[1].0), "`all` keeps time order");
        assert!(recording("no/such/file.jsonl").is_err());
    }

    #[test]
    fn slots_stack_then_wrap() {
        let main = MonitorInfo { name: None, position: [0, 0], size: [3840, 2160] };
        let s = slots(&main, 1.5, [1920, 550], 3);
        assert_eq!(s[0].size, [2880, 825]);
        assert_eq!(s[0].position, [40, 40]);
        assert_eq!(s[1].position, [40, 905]);
        // A third doesn't fit below (40 + 2 × 865 + 825 > 2160): back to the top, shifted right.
        assert_eq!(s[2].position, [80, 40]);
    }
}
