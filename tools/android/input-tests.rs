//! Dependency-free tests of the actual router and gesture implementation.
#[path = "../../vendor/winit/src/platform/android_cad.rs"]
mod cad;
#[path = "../../apps/solidcraft-android/src/finger_gestures.rs"]
mod finger_gestures;
#[test]
fn exclusive_capture_survives_crossing_and_pen_contact() {
    use cad::*;
    reset();
    viewport(Some([0.0, 0.0, 100.0, 100.0]));
    let e = Contact { device: 1, id: 1, source: Source::Finger, phase: Phase::Down, position: [20.0, 20.0], pressure: 1.0, time_ns: 1, buttons: 0 };
    assert!(route(e));
    assert!(!route(Contact { id: 2, source: Source::Stylus, ..e }));
    assert!(route(Contact { phase: Phase::Move, position: [200.0, 200.0], ..e }));
    assert!(!route(Contact { id: 3, position: [200.0, 200.0], ..e }));
    assert!(!route(Contact { id: 3, phase: Phase::Move, ..e }));
    assert_eq!(drain().len(), 2);
    assert!(route(Contact { phase: Phase::Cancel, ..e }));
    assert!(!route(Contact { phase: Phase::Move, ..e }));
    reset();
    assert!(drain().is_empty());
    assert!(route(e));
    for _ in 0..5000 {
        assert!(route(Contact { phase: Phase::Move, ..e }));
    }
    assert!(drain().iter().any(|e| e.phase == Phase::Cancel));
    assert!(route(Contact { phase: Phase::Up, ..e }));
    reset();
}
