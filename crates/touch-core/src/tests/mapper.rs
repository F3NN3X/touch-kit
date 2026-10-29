use crate::config::{CalPoint, PanelConfig, Rotation};
use crate::mapper::Mapper;

fn panel(rotation: Rotation, flip_x: bool, flip_y: bool) -> PanelConfig {
    PanelConfig { width_px: 101, height_px: 51, rotation, flip_x, flip_y, calibration: None }
}

const TL: (f32, f32) = (0.0, 0.0);
const TR: (f32, f32) = (1.0, 0.0);
const BR: (f32, f32) = (1.0, 1.0);
const BL: (f32, f32) = (0.0, 1.0);

/// Where each touch corner (TL, TR, BR, BL) lands, in px on a 101x51 panel.
fn corners(cfg: &PanelConfig) -> Vec<(f32, f32)> {
    let m = Mapper::new(cfg).unwrap();
    [TL, TR, BR, BL].iter().map(|&(x, y)| m.map(x, y)).collect()
}

const PX_TL: (f32, f32) = (0.0, 0.0);
const PX_TR: (f32, f32) = (100.0, 0.0);
const PX_BR: (f32, f32) = (100.0, 50.0);
const PX_BL: (f32, f32) = (0.0, 50.0);

#[test]
fn rotations_move_corners_clockwise() {
    assert_eq!(corners(&panel(Rotation::R0, false, false)), vec![PX_TL, PX_TR, PX_BR, PX_BL]);
    assert_eq!(corners(&panel(Rotation::R90, false, false)), vec![PX_TR, PX_BR, PX_BL, PX_TL]);
    assert_eq!(corners(&panel(Rotation::R180, false, false)), vec![PX_BR, PX_BL, PX_TL, PX_TR]);
    assert_eq!(corners(&panel(Rotation::R270, false, false)), vec![PX_BL, PX_TL, PX_TR, PX_BR]);
}

#[test]
fn flips_mirror_after_rotation() {
    assert_eq!(corners(&panel(Rotation::R0, true, false)), vec![PX_TR, PX_TL, PX_BL, PX_BR]);
    assert_eq!(corners(&panel(Rotation::R0, false, true)), vec![PX_BL, PX_BR, PX_TR, PX_TL]);
    assert_eq!(corners(&panel(Rotation::R0, true, true)), vec![PX_BR, PX_BL, PX_TL, PX_TR]);
    // R90 then flip X: TL → TR (rotate) → TL (flip).
    assert_eq!(corners(&panel(Rotation::R90, true, false)), vec![PX_TL, PX_BL, PX_BR, PX_TR]);
}

#[test]
fn centre_maps_to_centre() {
    let m = Mapper::new(&panel(Rotation::R0, false, false)).unwrap();
    assert_eq!(m.map(0.5, 0.5), (50.0, 25.0));
}

#[test]
fn calibration_recovers_a_known_affine() {
    // Panel px = 1800·u + 40, 500·v + 20 + 10·u (slight shear), as if the digitizer overhangs.
    let f = |u: f32, v: f32| [1800.0 * u + 40.0, 500.0 * v + 20.0 + 10.0 * u];
    let pts = [(0.1, 0.1), (0.9, 0.1), (0.9, 0.9), (0.1, 0.9)].map(|(u, v)| CalPoint { touch: [u, v], panel: f(u, v) });
    let cfg = PanelConfig { width_px: 1920, height_px: 550, calibration: Some(pts), ..Default::default() };
    let m = Mapper::new(&cfg).unwrap();
    for (u, v) in [(0.5, 0.5), (0.25, 0.75), (0.0, 0.0)] {
        let (x, y) = m.map(u, v);
        let [ex, ey] = f(u, v);
        assert!((x - ex).abs() < 0.05 && (y - ey).abs() < 0.05, "({u},{v}) -> ({x},{y}), want ({ex},{ey})");
    }
}

#[test]
fn degenerate_calibration_is_rejected() {
    let p = CalPoint { touch: [0.5, 0.5], panel: [10.0, 10.0] };
    let cfg = PanelConfig { calibration: Some([p; 4]), ..Default::default() };
    assert!(Mapper::new(&cfg).is_err());
}

#[test]
fn zero_size_panel_is_rejected() {
    let cfg = PanelConfig { width_px: 0, ..Default::default() };
    assert!(Mapper::new(&cfg).is_err());
}

#[test]
fn calibration_error_is_zero_for_an_exact_fit_and_measures_a_bad_tap() {
    use crate::mapper::calibration_error;
    // Touches at 10 % insets, panel points an exact affine image of them (scale + offset).
    let at = |u: f32, v: f32| CalPoint { touch: [u, v], panel: [20.0 + 1000.0 * u, 5.0 + 500.0 * v] };
    let exact = [at(0.1, 0.1), at(0.9, 0.1), at(0.9, 0.9), at(0.1, 0.9)];
    assert!(calibration_error(&exact).unwrap() < 0.01);
    let mut off = exact;
    off[2].panel[0] += 40.0; // one tap 40 px off
    let e = calibration_error(&off).unwrap();
    assert!(e > 5.0 && e < 40.0, "{e}");
    assert!(calibration_error(&[at(0.5, 0.5); 4]).is_err(), "all four taps in one place");
}
