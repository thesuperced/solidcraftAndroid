//! Contextual CAD keypad. Captured taps enter RawInput without defocusing the editor.
#[derive(Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Keypad {
    #[serde(skip)]
    pub shift: bool,
    #[serde(skip)]
    pub ctrl: bool,
    abc: bool,
    position: Option<[f32; 2]>,
    #[serde(skip)]
    pub pen: bool,
    #[serde(skip)]
    drawn_position: egui::Pos2,
    #[serde(skip)]
    drag_base: egui::Pos2,
    #[serde(skip)]
    pub pending: Vec<egui::Event>,
    #[serde(skip)]
    owner: Option<egui::Id>,
    #[serde(skip)]
    key_rects: std::collections::BTreeMap<String, egui::Rect>,
    #[serde(skip)]
    captured_key: Option<String>,
    #[serde(skip)]
    capture_origin: egui::Pos2,
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
            } else if self.shift && text.chars().count() == 1 {
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
            if let egui::Event::PointerMoved(pos) = &event
                && self.captured_key.as_deref() == Some("__move")
            {
                if self.pen {
                    let at = self.drag_base + (*pos - self.capture_origin);
                    self.position = Some([at.x, at.y]);
                }
                ctx.request_repaint();
                continue;
            }
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
                    self.capture_origin = pos;
                    self.drag_base = self.drawn_position;
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
                if label != "__move" && pos.distance(self.capture_origin) <= ctx.options(|o| o.input_options.max_click_dist).max(8.0) {
                    self.activate(&label);
                }
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
        let screen = ctx.content_rect();
        let width = 260.0;
        let height = if self.abc { 444.0 } else { 244.0 };
        let default = egui::pos2(screen.right() - width - 12.0, screen.bottom() - height - 12.0);
        let at = self.position.map(|p| egui::pos2(p[0], p[1])).unwrap_or(default);
        let position = egui::pos2(
            at.x.clamp(screen.left(), (screen.right() - width).max(screen.left())),
            at.y.clamp(screen.top(), (screen.bottom() - height).max(screen.top())),
        );
        self.drawn_position = position;
        egui::Window::new("Value input")
            .id(egui::Id::new("android_value_keypad"))
            .fixed_pos(position)
            .resizable(false)
            .collapsible(false)
            .title_bar(false)
            .show(&ctx, |ui| {
                self.key_rects.clear();
                ui.horizontal(|ui| {
                    let r = ui.add_sized([194.0, 22.0], egui::Button::new("Move keypad"));
                    self.key_rects.insert("__move".into(), r.rect);
                    let r = ui.add_sized([46.0, 22.0], egui::Button::new("X"));
                    self.key_rects.insert("Esc".into(), r.rect);
                    if r.clicked() {
                        clicked = true;
                        self.activate("Esc");
                    }
                });
                let defaults: Vec<&str> = if self.abc {
                    "a b c d e f g h i j k l m n o p q r s t u v w x y z ä ö ü _ / : Space Shift 123 Backspace Clear Enter"
                        .split_whitespace()
                        .collect()
                } else {
                    "7 8 9 / Backspace 4 5 6 * Clear 1 2 3 - mm 0 . + Enter ABC".split_whitespace().collect()
                };
                let labels: Vec<String> = defaults.iter().map(|s| (*s).to_owned()).collect();
                egui::Grid::new("cad_keys").num_columns(5).spacing([3.0, 6.0]).show(ui, |ui| {
                    for (index, label) in labels.iter().enumerate() {
                        let active = (label == "Shift" && self.shift) || (label == "Ctrl" && self.ctrl) || self.captured_key.as_ref() == Some(label);
                        let display = match label.as_str() {
                            "Backspace" => "Bksp",
                            "Delete" => "Del",
                            "Enter" => "OK",
                            "Esc" => "X",
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
    fn german_letters_and_shift_enter_unicode_text() {
        let mut pad = Keypad::default();
        for label in ["ä", "ö", "ü", "Shift", "ä", "ö", "ü"] {
            pad.activate(label);
        }
        let text: String = pad.pending.iter().filter_map(|e| if let egui::Event::Text(text) = e { Some(text.as_str()) } else { None }).collect();
        assert_eq!(text, "äöüÄÖÜ");
    }
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
        for removed in ["deg", "Undo", "Redo", "Delete"] {
            assert!(!pad.key_rects.contains_key(removed));
        }
        pad.pen = true;
        let origin = pad.key_rects.get("__move").unwrap().center();
        let old_position = pad.drawn_position;
        let moved = origin + egui::vec2(-100.0, -80.0);
        let _ = frame(
            &mut pad,
            &mut value,
            vec![egui::Event::PointerButton { pos: origin, button: egui::PointerButton::Primary, pressed: true, modifiers: Default::default() }],
            false,
        );
        let _ = frame(&mut pad, &mut value, vec![egui::Event::PointerMoved(moved)], false);
        let _ = frame(
            &mut pad,
            &mut value,
            vec![egui::Event::PointerButton { pos: moved, button: egui::PointerButton::Primary, pressed: false, modifiers: Default::default() }],
            false,
        );
        assert_eq!(pad.position, Some([old_position.x - 100.0, old_position.y - 80.0]));
        assert_eq!(value, "7", "moving the keypad must never type its handle label");
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
    fn popup_keeps_focus_through_digits_backspace_and_confirm() {
        let ctx = egui::Context::default();
        let mut pad = Keypad::default();
        let id = egui::Id::new("inline-value");
        let mut text = String::new();
        let mut closed = false;
        let mut committed = false;
        let mut frame = |pad: &mut Keypad, events: Vec<egui::Event>, focus: bool| {
            let mut raw = egui::RawInput {
                focused: true,
                screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(900.0, 900.0))),
                events,
                ..Default::default()
            };
            pad.intercept_pointer(&ctx, &mut raw);
            pad.restore_focus(&ctx);
            pad.input(&mut raw, Default::default());
            let mut output = ctx.run_ui(raw, |ui| {
                pad.show(ui);
                let r = ui.add(egui::TextEdit::singleline(&mut text).id(id));
                if focus {
                    r.request_focus();
                }
                if r.lost_focus() {
                    closed = true;
                    committed = ui.input(|i| i.key_pressed(egui::Key::Enter));
                }
            });
            output.textures_delta.clear();
        };
        frame(&mut pad, Vec::new(), true);
        frame(&mut pad, Vec::new(), false);
        for label in ["1", "2", "Backspace", "3", "Enter"] {
            let pos = pad.key_rects.get(label).unwrap().center();
            for pressed in [true, false] {
                frame(
                    &mut pad,
                    vec![
                        egui::Event::PointerMoved(pos),
                        egui::Event::PointerButton { pos, button: egui::PointerButton::Primary, pressed, modifiers: Default::default() },
                    ],
                    false,
                );
            }
        }
        assert_eq!(text, "13");
        assert!(closed && committed);
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
