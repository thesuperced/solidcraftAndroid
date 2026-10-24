//! Contextual CAD keypad. Captured taps enter RawInput without defocusing the editor.
#[derive(Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Keypad {
    #[serde(skip)]
    pub shift: bool,
    #[serde(skip)]
    pub ctrl: bool,
    abc: bool,
    #[serde(skip)]
    pub pending: Vec<egui::Event>,
    #[serde(skip)]
    owner: Option<egui::Id>,
    #[serde(skip)]
    key_rects: std::collections::BTreeMap<String, egui::Rect>,
    #[serde(skip)]
    captured_key: Option<String>,
}

impl Keypad {
    pub fn reset_focus(&mut self) {
        self.owner = None;
        self.pending.clear();
        self.captured_key = None;
        self.shift = false;
        self.ctrl = false;
    }
    pub fn restore_focus(&self, ctx: &egui::Context) {
        if !self.pending.is_empty()
            && let Some(owner) = self.owner
        {
            ctx.memory_mut(|m| m.request_focus(owner));
        }
    }
    fn key(&mut self, key: egui::Key, modifiers: egui::Modifiers) {
        for pressed in [true, false] {
            self.pending.push(egui::Event::Key { key, physical_key: None, pressed, repeat: false, modifiers });
        }
    }
    fn activate(&mut self, label: &str) {
        let plain = egui::Modifiers::default();
        let command = egui::Modifiers { ctrl: true, command: true, ..plain };
        match label {
            "123" => self.abc = false,
            "ABC" => self.abc = true,
            "Shift" => self.shift = !self.shift,
            "Ctrl" => self.ctrl = !self.ctrl,
            "Esc" => self.key(egui::Key::Escape, plain),
            "Undo" => self.key(egui::Key::Z, command),
            "Redo" => self.key(egui::Key::Z, egui::Modifiers { shift: true, ..command }),
            "Clear" => {
                self.key(egui::Key::A, command);
                self.key(egui::Key::Backspace, plain);
            }
            "Backspace" => self.key(egui::Key::Backspace, plain),
            "Delete" => self.key(egui::Key::Delete, plain),
            "Enter" => self.key(egui::Key::Enter, plain),
            "Tab" => self.key(egui::Key::Tab, plain),
            text => self.pending.push(egui::Event::Text(if text == "Space" {
                " ".into()
            } else if self.shift && text.len() == 1 && text.is_ascii() {
                text.to_uppercase()
            } else {
                text.into()
            })),
        }
    }
    /// Capture keypad pointer clicks before egui can treat them as clicks outside the active
    /// editor. The key rectangles come from the preceding frame, when the keypad was painted.
    pub fn intercept_pointer(&mut self, ctx: &egui::Context, raw: &mut egui::RawInput) {
        if self.owner.is_none() || !ctx.egui_wants_keyboard_input() {
            self.captured_key = None;
            return;
        }
        let mut kept = Vec::with_capacity(raw.events.len());
        for event in raw.events.drain(..) {
            let pointer = match &event {
                egui::Event::PointerButton { pos, button: egui::PointerButton::Primary, pressed, .. } => Some((*pos, *pressed)),
                _ => None,
            };
            let Some((pos, pressed)) = pointer else {
                kept.push(event);
                continue;
            };
            if pressed {
                let hit = self.key_rects.iter().find(|(_, rect)| rect.contains(pos)).map(|(label, _)| label.clone());
                if let Some(label) = hit {
                    self.captured_key = Some(label);
                    if let Some(owner) = self.owner {
                        ctx.memory_mut(|m| m.request_focus(owner));
                    }
                    ctx.request_repaint();
                    continue;
                }
            } else if let Some(label) = self.captured_key.take() {
                // The floating window can shift by a few points as egui settles its size between
                // frames. The press established the intended key, so complete that captured tap
                // on release instead of re-hit-testing against a newly laid-out rectangle.
                self.activate(&label);
                if let Some(owner) = self.owner {
                    ctx.memory_mut(|m| m.request_focus(owner));
                }
                ctx.request_repaint();
                continue;
            }
            kept.push(event);
        }
        raw.events = kept;
    }
    pub fn input(&mut self, raw: &mut egui::RawInput, mut physical: egui::Modifiers) {
        physical.shift |= self.shift;
        physical.ctrl |= self.ctrl;
        physical.command |= self.ctrl;
        raw.events.insert(0, egui::Event::ModifiersChanged(physical));
        for event in &mut raw.events {
            if let egui::Event::PointerButton { modifiers, .. } | egui::Event::Key { modifiers, .. } | egui::Event::ModifiersChanged(modifiers) =
                event
            {
                modifiers.shift |= self.shift;
                modifiers.ctrl |= self.ctrl;
                modifiers.command |= self.ctrl;
            }
        }
        raw.events.append(&mut self.pending);
    }
    pub fn show(&mut self, parent: &mut egui::Ui) {
        let ctx = parent.ctx().clone();
        let focused = ctx.memory(|m| m.focused());
        if ctx.egui_wants_keyboard_input() {
            self.owner = focused;
        }
        let wanted = ctx.egui_wants_keyboard_input();
        if !wanted {
            self.owner = None;
            self.captured_key = None;
            self.key_rects.clear();
            return;
        }
        let mut clicked = false;
        egui::Window::new("Value input")
            .id(egui::Id::new("android_value_keypad"))
            .anchor(egui::Align2::RIGHT_CENTER, [-16.0, 0.0])
            .resizable(false)
            .collapsible(false)
            .title_bar(false)
            .show(&ctx, |ui| {
                let defaults: Vec<&str> = if self.abc {
                    "a b c d e f g h i j k l m n o p q r s t u v w x y z _ / : Space 123 Backspace Clear Esc Enter".split_whitespace().collect()
                } else {
                    "7 8 9 / Backspace 4 5 6 * Clear 1 2 3 - mm 0 . + deg Enter Esc Undo Redo Delete ABC".split_whitespace().collect()
                };
                let labels: Vec<String> = defaults.iter().map(|s| (*s).to_owned()).collect();
                self.key_rects.clear();
                egui::Grid::new("cad_keys").num_columns(5).spacing([3.0, 6.0]).show(ui, |ui| {
                    for (index, label) in labels.iter().enumerate() {
                        let active = (label == "Shift" && self.shift) || (label == "Ctrl" && self.ctrl) || self.captured_key.as_ref() == Some(label);
                        let display = match label.as_str() {
                            "Backspace" => "Bksp",
                            "Delete" => "Del",
                            "Enter" => "✓",
                            "Esc" => "✕",
                            other => other,
                        };
                        let response = ui.add_sized([46.0, 44.0], egui::Button::new(display).selected(active));
                        self.key_rects.insert(label.clone(), response.rect);
                        if response.clicked() {
                            clicked = true;
                            self.activate(label);
                        }
                        if index % 5 == 4 {
                            ui.end_row();
                        }
                    }
                });
            });
        if clicked {
            if let Some(owner) = self.owner {
                ctx.memory_mut(|m| m.request_focus(owner));
            }
            ctx.request_repaint();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn touching_a_key_restores_text_focus_and_replaces_selection() {
        let ctx = egui::Context::default();
        let mut pad = Keypad::default();
        let mut value = "12".to_owned();
        let field = egui::Id::new("dimension");
        let frame = |pad: &mut Keypad, value: &mut String, events: Vec<egui::Event>, focus: bool| {
            let mut raw = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(900.0, 900.0))),
                focused: true,
                events,
                ..Default::default()
            };
            pad.intercept_pointer(&ctx, &mut raw);
            pad.restore_focus(&ctx);
            pad.input(&mut raw, egui::Modifiers::default());
            let delivered = raw.events.clone();
            let mut output = ctx.run_ui(raw, |ui| {
                pad.show(ui);
                let response = ui.add(egui::TextEdit::singleline(value).id(field));
                if focus {
                    response.request_focus();
                }
            });
            output.textures_delta.clear();
            delivered
        };
        let _ = frame(&mut pad, &mut value, Vec::new(), true);
        pad.key(egui::Key::A, egui::Modifiers { ctrl: true, command: true, ..Default::default() });
        let _ = frame(&mut pad, &mut value, Vec::new(), false);
        let pos = pad.key_rects.get("7").unwrap().center();
        let _ = frame(
            &mut pad,
            &mut value,
            vec![
                egui::Event::PointerMoved(pos),
                egui::Event::PointerButton { pos, button: egui::PointerButton::Primary, pressed: true, modifiers: Default::default() },
            ],
            false,
        );
        assert_eq!(pad.captured_key.as_deref(), Some("7"));
        assert_eq!(ctx.memory(|m| m.focused()), Some(field));
        let delivered = frame(
            &mut pad,
            &mut value,
            vec![
                egui::Event::PointerMoved(pos),
                egui::Event::PointerButton { pos, button: egui::PointerButton::Primary, pressed: false, modifiers: Default::default() },
            ],
            false,
        );
        assert!(pad.captured_key.is_none());
        assert!(delivered.iter().any(|event| matches!(event, egui::Event::Text(text) if text == "7")));
        let _ = frame(&mut pad, &mut value, Vec::new(), false);
        assert_eq!(value, "7");
        assert_eq!(ctx.memory(|m| m.focused()), Some(field));
    }
    #[test]
    fn keypad_text_edits_the_focused_widget() {
        let ctx = egui::Context::default();
        let mut pad = Keypad::default();
        let mut value = String::new();
        let field = egui::Id::new("dimension");
        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
            ui.add(egui::TextEdit::singleline(&mut value).id(field)).request_focus();
        });
        output.textures_delta.clear();
        pad.pending.push(egui::Event::Text("-12.5".into()));
        let mut raw = egui::RawInput::default();
        pad.input(&mut raw, egui::Modifiers::default());
        let mut output = ctx.run_ui(raw, |ui| {
            ui.add(egui::TextEdit::singleline(&mut value).id(field));
        });
        output.textures_delta.clear();
        assert_eq!(value, "-12.5");
        pad.key(egui::Key::Backspace, egui::Modifiers::default());
        let mut raw = egui::RawInput::default();
        pad.input(&mut raw, egui::Modifiers::default());
        let mut output = ctx.run_ui(raw, |ui| {
            ui.add(egui::TextEdit::singleline(&mut value).id(field));
        });
        output.textures_delta.clear();
        assert_eq!(value, "-12.");
        assert_eq!(ctx.memory(|m| m.focused()), Some(field));
    }
    #[test]
    fn settings_never_persist_modifiers_or_input() {
        let mut pad = Keypad { shift: true, ctrl: true, ..Default::default() };
        pad.pending.push(egui::Event::Text("private filename".into()));
        let encoded = serde_json::to_string(&pad).unwrap();
        assert!(!encoded.contains("private filename"));
        let loaded: Keypad = serde_json::from_str(&encoded).unwrap();
        assert!(!loaded.shift && !loaded.ctrl && loaded.pending.is_empty());
    }
    #[test]
    fn modifiers_combine_with_physical_keyboard() {
        let mut pad = Keypad { shift: true, ..Default::default() };
        let mut raw = egui::RawInput::default();
        pad.input(&mut raw, egui::Modifiers { ctrl: true, ..Default::default() });
        assert!(matches!(raw.events.first(), Some(egui::Event::ModifiersChanged(m)) if m.shift && m.ctrl));
    }
}
