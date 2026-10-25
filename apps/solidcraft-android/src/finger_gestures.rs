//! Finger-only state machine; no egui pointer state or pen contacts enter this map.
use crate::cad::{Contact, Phase};
use std::collections::BTreeMap;

#[derive(Default)]
pub struct Gestures {
    contacts: BTreeMap<(i32, i32), ([f32; 2], [f32; 2])>,
    multi: bool,
    peak: usize,
    started: i64,
    tap_valid: bool,
    three_tap: bool,
    pub tap_slop: f32,
}

pub type Motion = ([f32; 2], bool, f32);

/// Fold the raw per-pointer moves queued between frames into camera updates.
/// Android reports both pointers for every two-finger move; replaying every sample through
/// `view.navigate` builds a visible backlog on slower tablets.
pub fn coalesce(motions: impl IntoIterator<Item = Motion>) -> Vec<Motion> {
    let mut batched: Vec<Motion> = Vec::new();
    for (delta, pan, zoom) in motions {
        if let Some((sum, previous_pan, product)) = batched.last_mut()
            && *previous_pan == pan
        {
            sum[0] += delta[0];
            sum[1] += delta[1];
            *product *= zoom;
        } else {
            batched.push((delta, pan, zoom));
        }
    }
    batched
}

impl Gestures {
    pub fn contact_count(&self) -> usize {
        self.contacts.len()
    }
    pub fn anchor(&self) -> Option<[f32; 2]> {
        let count = self.contacts.len();
        if count == 0 {
            return None;
        }
        let sum = self.contacts.values().fold([0.0, 0.0], |sum, (_, p)| [sum[0] + p[0], sum[1] + p[1]]);
        Some([sum[0] / count as f32, sum[1] / count as f32])
    }
    pub fn reset(&mut self) {
        self.contacts.clear();
        self.multi = false;
        self.peak = 0;
        self.tap_valid = false;
        self.three_tap = false;
    }
    pub fn take_three_tap(&mut self) -> bool {
        std::mem::take(&mut self.three_tap)
    }
    pub fn event(&mut self, e: Contact) -> Option<([f32; 2], bool, f32)> {
        if e.source != crate::cad::Source::Finger {
            return None;
        }
        let key = (e.device, e.id);
        if e.position.iter().any(|v| !v.is_finite()) {
            self.reset();
            return None;
        }
        match e.phase {
            Phase::Down => {
                if self.contacts.is_empty() {
                    self.peak = 0;
                    self.started = e.time_ns;
                    self.tap_valid = true;
                }
                self.contacts.entry(key).or_insert((e.position, e.position));
                self.peak = self.peak.max(self.contacts.len());
                self.multi |= self.contacts.len() >= 2;
                None
            }
            Phase::Cancel => {
                self.reset();
                None
            }
            Phase::Up => {
                if let Some((origin, _)) = self.contacts.remove(&key) {
                    self.tap_valid &= (e.position[0] - origin[0]).hypot(e.position[1] - origin[1]) <= self.tap_slop.max(8.0);
                } else {
                    return None;
                }
                if self.contacts.is_empty() {
                    self.three_tap |= self.peak == 3
                        && self.tap_valid
                        && e.time_ns.checked_sub(self.started).is_some_and(|elapsed| (0..=500_000_000).contains(&elapsed));
                    self.multi = false;
                }
                None
            }
            Phase::Move => {
                let old: Vec<_> = self.contacts.values().map(|(_, p)| *p).collect();
                let contact = self.contacts.get_mut(&key)?;
                let was = contact.1;
                contact.1 = e.position;
                self.tap_valid &= (e.position[0] - contact.0[0]).hypot(e.position[1] - contact.0[1]) <= self.tap_slop.max(8.0);
                if self.peak >= 3 {
                    return None;
                }
                if old.len() == 1 && !self.multi {
                    let dx = e.position[0] - contact.0[0];
                    let dy = e.position[1] - contact.0[1];
                    if dx * dx + dy * dy < 64.0 {
                        return None;
                    }
                    return Some(([e.position[0] - was[0], e.position[1] - was[1]], false, 1.0));
                }
                if old.len() != 2 {
                    return None;
                }
                let new: Vec<_> = self.contacts.values().map(|(_, p)| *p).collect();
                let (Some(a), Some(b), Some(c), Some(d)) = (old.first(), old.get(1), new.first(), new.get(1)) else { return None };
                let span = |a: &[f32; 2], b: &[f32; 2]| ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2)).sqrt();
                let zoom = if span(c, d) > 1.0 { span(a, b) / span(c, d) } else { 1.0 };
                Some(([(c[0] + d[0] - a[0] - b[0]) * 0.5, (c[1] + d[1] - a[1] - b[1]) * 0.5], true, zoom))
            }
            Phase::Hover | Phase::Leave => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cad::Source;
    fn event(id: i32, phase: Phase, x: f32, y: f32) -> Contact {
        Contact { device: 1, id, source: Source::Finger, phase, position: [x, y], pressure: 1.0, ..Default::default() }
    }
    #[test]
    fn three_finger_tap_uses_one_action_but_drag_cancel_and_four_fingers_do_not() {
        let mut g = Gestures::default();
        for id in 1..=3 {
            g.event(event(id, Phase::Down, id as f32 * 30.0, 0.0));
        }
        for id in [2, 1, 3] {
            g.event(event(id, Phase::Up, id as f32 * 30.0, 0.0));
        }
        assert!(g.take_three_tap());
        assert!(!g.take_three_tap());
        for id in 1..=3 {
            g.event(event(id, Phase::Down, id as f32 * 30.0, 0.0));
        }
        assert!(g.event(event(1, Phase::Move, 60.0, 0.0)).is_none());
        for id in 1..=3 {
            g.event(event(id, Phase::Up, id as f32 * 30.0, 0.0));
        }
        assert!(!g.take_three_tap());
        for id in 1..=4 {
            g.event(event(id, Phase::Down, id as f32 * 30.0, 0.0));
        }
        for id in 1..=4 {
            g.event(event(id, Phase::Up, id as f32 * 30.0, 0.0));
        }
        assert!(!g.take_three_tap());
        for id in 1..=3 {
            g.event(event(id, Phase::Down, id as f32 * 30.0, 0.0));
        }
        g.event(event(1, Phase::Cancel, 30.0, 0.0));
        assert!(!g.take_three_tap());
    }
    #[test]
    fn tap_slop_orbit_pinch_and_lift() {
        let mut g = Gestures::default();
        assert!(g.event(event(1, Phase::Down, 0.0, 0.0)).is_none());
        assert!(g.event(event(1, Phase::Move, 2.0, 0.0)).is_none());
        assert_eq!(g.event(event(1, Phase::Move, 10.0, 0.0)), Some(([8.0, 0.0], false, 1.0)));
        assert!(g.event(event(2, Phase::Down, 30.0, 0.0)).is_none());
        let (_, pan, zoom) = g.event(event(2, Phase::Move, 50.0, 0.0)).unwrap();
        assert!(pan);
        assert_eq!(zoom, 0.5);
        g.event(event(2, Phase::Up, 50.0, 0.0));
        assert!(g.event(event(1, Phase::Move, 60.0, 0.0)).is_none());
        g.event(event(1, Phase::Up, 60.0, 0.0));
        assert!(g.contacts.is_empty());
    }
    #[test]
    fn cancel_and_invalid_coordinates_reset() {
        let mut g = Gestures::default();
        g.event(event(1, Phase::Down, 0.0, 0.0));
        g.event(event(1, Phase::Cancel, 0.0, 0.0));
        assert!(g.event(event(1, Phase::Move, 40.0, 0.0)).is_none());
        g.event(event(1, Phase::Down, 0.0, 0.0));
        g.event(event(1, Phase::Move, f32::NAN, 0.0));
        assert!(g.contacts.is_empty());
    }

    #[test]
    fn queued_pointer_moves_become_one_camera_update() {
        let motions = (0..120).map(|_| ([0.5, -0.25], true, 0.999));
        let batched = coalesce(motions);
        assert_eq!(batched.len(), 1);
        assert_eq!(batched[0].0, [60.0, -30.0]);
        assert!(batched[0].1);
        assert!((batched[0].2 - 0.999_f32.powi(120)).abs() < 1e-5);
    }
}
