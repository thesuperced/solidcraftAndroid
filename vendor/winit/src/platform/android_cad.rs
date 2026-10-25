//! Typed Android CAD input transport. Contacts keep their initial owner until release.
use std::collections::BTreeMap;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
static PEN_CANCELED: AtomicBool = AtomicBool::new(false);
static CAD_IME_POLICY: AtomicBool = AtomicBool::new(false);
pub fn suppress_automatic_ime() -> bool {
    CAD_IME_POLICY.load(Ordering::Relaxed)
}
pub fn cancel_pen() {
    PEN_CANCELED.store(true, Ordering::Relaxed);
}
pub fn take_pen_cancel() -> bool {
    PEN_CANCELED.swap(false, Ordering::Relaxed)
}
/// Ignore the rest of an active pen contact after off-hand cancellation, until release.
pub fn cancel_modeling() {
    if let Ok(mut state) = ROUTER.lock() {
        let mut canceled = false;
        for owner in state.owners.values_mut() {
            if *owner == Owner::Modeling {
                *owner = Owner::Rejected;
                canceled = true;
            }
        }
        if canceled {
            cancel_pen();
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Source {
    Finger,
    Stylus,
    Eraser,
    Mouse,
    #[default]
    Unknown,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Phase {
    #[default]
    Down,
    Move,
    Up,
    Cancel,
    Hover,
    Leave,
}
#[derive(Clone, Copy, Debug, Default)]
pub struct Contact {
    pub device: i32,
    pub id: i32,
    pub source: Source,
    pub phase: Phase,
    pub position: [f32; 2],
    pub pressure: f32,
    pub time_ns: i64,
    pub buttons: u32,
    pub tilt: f32,
    pub orientation: f32,
    pub historical: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Owner {
    Ui,
    Modeling,
    Camera,
    Control(u8),
    Rejected,
}
#[derive(Clone, Copy, Debug)]
pub struct ControlEvent {
    pub control: u8,
    pub contact: Contact,
}
pub struct CadInputRouter {
    event_count: u64,
    cancel_count: u64,
    history_count: u64,
    excluded: Vec<[f32; 4]>,
    last: Option<Contact>,
    region: Option<[f32; 4]>,
    owners: BTreeMap<(i32, i32), Owner>,
    events: Vec<Contact>,
    controls: Vec<(u8, [f32; 4])>,
    control_events: Vec<ControlEvent>,
    pointer_source: Source,
    hit_test: Option<Box<dyn Fn([f32; 2]) -> bool + Send>>,
}
static ROUTER: Mutex<CadInputRouter> = Mutex::new(CadInputRouter::new());
impl Default for CadInputRouter {
    fn default() -> Self {
        Self::new()
    }
}
fn contains([x0, y0, x1, y1]: [f32; 4], [x, y]: [f32; 2]) -> bool {
    x.is_finite() && y.is_finite() && x >= x0 && y >= y0 && x <= x1 && y <= y1
}
impl CadInputRouter {
    pub const fn new() -> Self {
        Self {
            event_count: 0,
            cancel_count: 0,
            history_count: 0,
            excluded: Vec::new(),
            last: None,
            region: None,
            owners: BTreeMap::new(),
            events: Vec::new(),
            controls: Vec::new(),
            control_events: Vec::new(),
            pointer_source: Source::Unknown,
            hit_test: None,
        }
    }
    pub fn owner(&self, device: i32, id: i32) -> Option<Owner> {
        self.owners.get(&(device, id)).copied()
    }
    pub fn route(&mut self, mut event: Contact) -> bool {
        if event.position.iter().any(|value| !value.is_finite()) {
            if matches!(event.source, Source::Stylus | Source::Eraser) {
                cancel_pen();
            }
            event.phase = Phase::Cancel;
            event.position = [0.0, 0.0];
        }
        self.event_count = self.event_count.saturating_add(1);
        if event.phase == Phase::Cancel {
            self.cancel_count = self.cancel_count.saturating_add(1);
        }
        if event.historical {
            self.history_count = self.history_count.saturating_add(1);
        }
        self.last = Some(event);
        if matches!(event.phase, Phase::Hover | Phase::Leave) {
            if event.source != Source::Finger {
                self.pointer_source = event.source;
            }
            return false;
        }
        let key = (event.device, event.id);
        if event.phase == Phase::Down {
            if self.owners.len() >= 128 && !self.owners.contains_key(&(event.device, event.id)) {
                return true;
            }
            let pen_down = self.owners.values().any(|o| *o == Owner::Modeling);
            let control = self.controls.iter().find(|(_, r)| contains(*r, event.position)).map(|(id, _)| *id);
            let viewport = self.region.is_some_and(|r| contains(r, event.position))
                && !self.excluded.iter().any(|r| contains(*r, event.position))
                && self.hit_test.as_ref().is_none_or(|test| test(event.position));
            let owner = if let Some(id) = control {
                Owner::Control(id)
            } else if !viewport {
                if pen_down && event.source == Source::Finger { Owner::Control(254) } else { Owner::Ui }
            } else {
                match event.source {
                    Source::Stylus | Source::Eraser => Owner::Modeling,
                    // Test-build policy: viewport fingers may navigate during pen contact.
                    // Explicit controls above always keep their independent captures.
                    Source::Finger => Owner::Camera,
                    Source::Mouse => Owner::Ui,
                    _ => Owner::Rejected,
                }
            };
            self.owners.entry(key).or_insert(owner);
        }
        let owner = self.owner(event.device, event.id).unwrap_or(Owner::Rejected);
        match owner {
            Owner::Camera => {
                if self.events.len() >= 4096 {
                    self.events.clear();
                    self.events.push(Contact { phase: Phase::Cancel, ..event });
                    for o in self.owners.values_mut() {
                        if *o == Owner::Camera {
                            *o = Owner::Rejected;
                        }
                    }
                } else {
                    self.events.push(event);
                }
            }
            Owner::Control(control) => {
                if self.control_events.len() >= 4096 {
                    self.control_events.clear();
                    // Cancel every control on overflow, including controls on other devices.
                    for (&(device, id), captured) in &mut self.owners {
                        if let Owner::Control(control) = *captured {
                            self.control_events.push(ControlEvent { control, contact: Contact { device, id, phase: Phase::Cancel, ..event } });
                            *captured = Owner::Rejected;
                        }
                    }
                } else {
                    self.control_events.push(ControlEvent { control, contact: event });
                }
            }
            Owner::Ui | Owner::Modeling => self.pointer_source = event.source,
            Owner::Rejected => {}
        }
        if matches!(event.phase, Phase::Up | Phase::Cancel) {
            self.owners.remove(&key);
        }
        matches!(owner, Owner::Camera | Owner::Control(_) | Owner::Rejected)
    }
}
pub fn controls(regions: Vec<(u8, [f32; 4])>) {
    if let Ok(mut state) = ROUTER.lock() {
        state.controls = regions;
    }
}
pub fn drain_controls() -> Vec<ControlEvent> {
    ROUTER.lock().map(|mut s| std::mem::take(&mut s.control_events)).unwrap_or_default()
}
pub fn pointer_source() -> Source {
    ROUTER.lock().map(|s| s.pointer_source).unwrap_or_default()
}
#[derive(Clone, Debug)]
pub struct Diagnostics {
    pub owners: Vec<(i32, i32, Owner)>,
    pub region: Option<[f32; 4]>,
    pub controls: Vec<(u8, [f32; 4])>,
    pub events: u64,
    pub canceled: u64,
    pub historical: u64,
}
pub fn diagnostics() -> Option<Diagnostics> {
    ROUTER.lock().ok().map(|s| Diagnostics {
        owners: s.owners.iter().map(|(&(device, id), owner)| (device, id, *owner)).collect(),
        region: s.region,
        controls: s.controls.clone(),
        events: s.event_count,
        canceled: s.cancel_count,
        historical: s.history_count,
    })
}
pub fn exclude(regions: Vec<[f32; 4]>) {
    if let Ok(mut state) = ROUTER.lock() {
        state.excluded = regions;
    }
}
pub fn last_contact() -> Option<Contact> {
    ROUTER.lock().ok().and_then(|state| state.last)
}
pub fn hit_test(test: Box<dyn Fn([f32; 2]) -> bool + Send>) {
    CAD_IME_POLICY.store(true, Ordering::Relaxed);
    if let Ok(mut state) = ROUTER.lock() {
        state.hit_test = Some(test);
    }
}

pub fn viewport(region: Option<[f32; 4]>) {
    if let Ok(mut state) = ROUTER.lock() {
        state.region = region;
    }
}
pub fn reset() {
    if let Ok(mut state) = ROUTER.lock() {
        state.owners.clear();
        state.events.clear();
        state.control_events.clear();
        state.pointer_source = Source::Unknown;
    }
}
pub fn drain() -> Vec<Contact> {
    ROUTER.lock().map(|mut state| std::mem::take(&mut state.events)).unwrap_or_default()
}
/// True means this contact is captured by navigation, a semantic control or palm rejection.
pub fn route(event: Contact) -> bool {
    let Ok(mut state) = ROUTER.lock() else { return false };
    state.route(event)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn contact(id: i32, source: Source, phase: Phase, x: f32) -> Contact {
        Contact { device: 7, id, source, phase, position: [x, 20.0], ..Default::default() }
    }
    fn router() -> CadInputRouter {
        let mut r = CadInputRouter::new();
        r.region = Some([0.0, 0.0, 100.0, 100.0]);
        r.controls = vec![(3, [110.0, 0.0, 160.0, 100.0])];
        r
    }
    #[test]
    fn capture_uses_ids_and_never_changes_on_crossing() {
        let mut r = router();
        assert!(r.route(contact(41, Source::Finger, Phase::Down, 20.0)));
        assert!(r.route(contact(9, Source::Finger, Phase::Down, 40.0)));
        assert!(r.route(contact(41, Source::Finger, Phase::Up, 200.0)));
        assert_eq!(r.owner(7, 9), Some(Owner::Camera));
        assert!(r.route(contact(9, Source::Finger, Phase::Move, 130.0)));
        assert!(r.control_events.is_empty());
        assert!(!r.route(contact(22, Source::Finger, Phase::Down, 200.0)));
        assert!(!r.route(contact(22, Source::Finger, Phase::Move, 30.0)));
    }
    #[test]
    fn off_hand_control_and_pen_have_independent_owners() {
        let mut r = router();
        assert!(!r.route(contact(8, Source::Stylus, Phase::Down, 10.0)));
        assert!(r.route(contact(4, Source::Finger, Phase::Down, 130.0)));
        assert_eq!(r.owner(7, 4), Some(Owner::Control(3)));
        assert!(r.route(contact(5, Source::Finger, Phase::Down, 30.0)));
        assert_eq!(r.owner(7, 5), Some(Owner::Camera));
        assert!(!r.route(contact(8, Source::Stylus, Phase::Move, 140.0)));
        assert!(r.route(contact(4, Source::Finger, Phase::Cancel, 10.0)));
        assert_eq!(r.owner(7, 8), Some(Owner::Modeling));
        assert_eq!(r.control_events.last().unwrap().contact.phase, Phase::Cancel);
    }
    #[test]
    fn eraser_mouse_hover_and_historical_metadata_are_distinct() {
        let mut r = router();
        let e = Contact { tilt: 0.4, orientation: -0.8, pressure: 0.3, time_ns: 91, buttons: 32, ..contact(7, Source::Eraser, Phase::Down, 20.0) };
        assert!(!r.route(e));
        assert!(!r.route(Contact { phase: Phase::Move, historical: true, ..e }));
        let last = r.last.unwrap();
        assert!(last.historical);
        assert_eq!((last.tilt, last.orientation, last.time_ns, last.buttons), (0.4, -0.8, 91, 32));
        assert!(!r.route(Contact { phase: Phase::Cancel, ..e }));
        assert!(r.owners.is_empty());
        assert!(!r.route(contact(1, Source::Mouse, Phase::Down, 30.0)));
        assert_eq!(r.owner(7, 1), Some(Owner::Ui));
        r.route(contact(6, Source::Stylus, Phase::Hover, 10.0));
        assert_eq!(r.owner(7, 6), None);
    }
}
