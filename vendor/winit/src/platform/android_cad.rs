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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    Finger,
    Stylus,
    Eraser,
    Mouse,
    Unknown,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Down,
    Move,
    Up,
    Cancel,
}
#[derive(Clone, Copy, Debug)]
pub struct Contact {
    pub device: i32,
    pub id: i32,
    pub source: Source,
    pub phase: Phase,
    pub position: [f32; 2],
    pub pressure: f32,
    pub time_ns: i64,
    pub buttons: u32,
}
struct Router {
    excluded: Vec<[f32; 4]>,
    last: Option<Contact>,
    region: Option<[f32; 4]>,
    owners: BTreeMap<(i32, i32), bool>,
    events: Vec<Contact>,
    hit_test: Option<Box<dyn Fn([f32; 2]) -> bool + Send>>,
}
static ROUTER: Mutex<Router> =
    Mutex::new(Router { excluded: Vec::new(), last: None, region: None, owners: BTreeMap::new(), events: Vec::new(), hit_test: None });
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
    }
}
pub fn drain() -> Vec<Contact> {
    ROUTER.lock().map(|mut state| std::mem::take(&mut state.events)).unwrap_or_default()
}
/// True means this contact belongs exclusively to camera navigation.
pub fn route(event: Contact) -> bool {
    let Ok(mut state) = ROUTER.lock() else { return false };
    state.last = Some(event);
    if event.source != Source::Finger {
        return false;
    }
    let key = (event.device, event.id);
    if event.phase == Phase::Down {
        let inside = state.region.is_some_and(|[x0, y0, x1, y1]| {
            let [x, y] = event.position;
            x.is_finite() && y.is_finite() && x >= x0 && y >= y0 && x <= x1 && y <= y1
        }) && !state
            .excluded
            .iter()
            .any(|r| event.position[0] >= r[0] && event.position[1] >= r[1] && event.position[0] <= r[2] && event.position[1] <= r[3])
            && state.hit_test.as_ref().is_none_or(|test| test(event.position));
        state.owners.entry(key).or_insert(inside);
    }
    let camera = state.owners.get(&key).copied().unwrap_or(false);
    if camera {
        if state.events.len() >= 4096 {
            state.events.clear();
            state.events.push(Contact { phase: Phase::Cancel, ..event });
        } else {
            state.events.push(event);
        }
    }
    if matches!(event.phase, Phase::Up | Phase::Cancel) {
        state.owners.remove(&key);
    }
    camera
}
