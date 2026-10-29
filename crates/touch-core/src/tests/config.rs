use crate::{CalPoint, Config};

#[test]
fn defaults_are_valid_and_unusable_values_are_named() {
    assert_eq!(Config::default().validate(), Vec::<String>::new());

    let mut cfg = Config::default();
    cfg.panel.width_px = 0;
    cfg.gestures.slop_px = 0.0;
    cfg.gestures.flick_min_px = 80.0; // above swipe_min_px 60
    cfg.gestures.pinch_min_scale = 1.5;
    let errors = cfg.validate();
    assert_eq!(errors.len(), 4, "{errors:?}");
    assert!(errors[0].starts_with("panel: "), "{errors:?}");
    assert!(errors.iter().any(|e| e.contains("swipe_min_px")), "{errors:?}");

    let mut cfg = Config::default();
    let p = CalPoint { touch: [0.5, 0.5], panel: [10.0, 10.0] };
    cfg.panel.calibration = Some([p; 4]);
    assert!(cfg.validate()[0].contains("collinear"), "repeated calibration points");
}
