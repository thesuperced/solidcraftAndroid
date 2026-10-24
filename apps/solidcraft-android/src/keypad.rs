//! Local CAD keypad. Button actions enter RawInput on the following frame.
#[derive(serde::Serialize, serde::Deserialize)]
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
    pub pinned: bool,
    collapsed: bool,
    opacity: f32,
    order: Vec<String>,
    selected: usize,
    #[serde(skip)]
    panel_rect: Option<egui::Rect>,
    #[cfg(test)]
    #[serde(skip)]
    key_rects: std::collections::BTreeMap<String, egui::Rect>,
}

impl Default for Keypad {
    fn default() -> Self {
        Self {
            shift: false,
            ctrl: false,
            abc: false,
            pending: Vec::new(),
            owner: None,
            pinned: true,
            collapsed: false,
            opacity: 1.0,
            order: Vec::new(),
            selected: 0,
            panel_rect: None,
            #[cfg(test)]
            key_rects: Default::default(),
        }
    }
}
impl Keypad {
    pub fn reset_focus(&mut self) {
        self.owner = None;
        self.pending.clear();
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
        if self.collapsed || (!self.pinned && !wanted) {
            let panel = egui::Panel::left("android_cad_keypad_collapsed").exact_size(56.0).show(parent, |ui| {
                if ui.button("CAD").clicked() {
                    self.collapsed = false;
                    self.pinned = true;
                }
            });
            self.panel_rect = Some(panel.response.rect);
            return;
        }
        if !wanted
            && ctx.input(|i| i.pointer.any_pressed() && i.pointer.interact_pos().is_some_and(|p| !self.panel_rect.is_some_and(|r| r.contains(p))))
        {
            self.owner = None;
        }
        let mut clicked = false;
        let panel = egui::Panel::left("android_cad_keypad").resizable(true).default_size(270.0).size_range(260.0..=500.0).show(parent, |ui| {
            ui.horizontal(|ui| {
                ui.checkbox(&mut self.pinned, "Pin");
                ui.checkbox(&mut self.collapsed, "Collapse");
                ui.checkbox(&mut self.abc, "ABC");
            });
            ui.collapsing("Keypad settings", |ui| {
                ui.add(egui::Slider::new(&mut self.opacity, 0.35..=1.0).text("Opacity"));
                if !self.order.is_empty() {
                    let max = self.order.len().saturating_sub(1);
                    ui.add(egui::Slider::new(&mut self.selected, 0..=max).text("Key position"));
                    if ui.button("Move selected key left").clicked() && self.selected > 0 && self.selected <= max {
                        self.order.swap(self.selected, self.selected - 1);
                        self.selected -= 1;
                    }
                    if ui.button("Reset layout").clicked() {
                        self.order.clear();
                    }
                }
            });
            if self.collapsed || (!self.pinned && !wanted && self.owner.is_none()) {
                return;
            }
            ui.set_opacity(if self.opacity.is_finite() { self.opacity.clamp(0.35, 1.0) } else { 1.0 });
            let defaults: Vec<&str> = if self.abc {
                "Shift Ctrl Esc Undo Redo a b c d e f g h i j k l m n o p q r s t u v w x y z _ / : Space Backspace Clear Enter Tab Delete"
                    .split_whitespace()
                    .collect()
            } else {
                "Shift Ctrl Esc Undo Redo 7 8 9 + Backspace 4 5 6 - Clear 1 2 3 . Enter 0 mm deg Tab Delete".split_whitespace().collect()
            };
            if !self.abc && (self.order.len() != defaults.len() || defaults.iter().any(|key| !self.order.iter().any(|saved| saved == key))) {
                self.order = defaults.iter().map(|s| (*s).to_owned()).collect();
            }
            let labels: Vec<String> = if self.abc { defaults.iter().map(|s| (*s).to_owned()).collect() } else { self.order.clone() };
            egui::Grid::new("cad_keys").num_columns(5).spacing([3.0, 6.0]).show(ui, |ui| {
                for (index, label) in labels.iter().enumerate() {
                    let active = (label == "Shift" && self.shift) || (label == "Ctrl" && self.ctrl);
                    let display = match label.as_str() {
                        "Backspace" => "Bksp",
                        "Delete" => "Del",
                        other => other,
                    };
                    let response = ui.add_sized([46.0, 44.0], egui::Button::new(display).selected(active));
                    #[cfg(test)]
                    self.key_rects.insert(label.clone(), response.rect);
                    if response.clicked() {
                        clicked = true;
                        let plain = egui::Modifiers::default();
                        let command = egui::Modifiers { ctrl: true, command: true, ..plain };
                        match label.as_str() {
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
                    if index % 5 == 4 {
                        ui.end_row();
                    }
                }
            });
        });
        self.panel_rect = Some(panel.response.rect);
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
            pad.restore_focus(&ctx);
            pad.input(&mut raw, egui::Modifiers::default());
            let mut output = ctx.run_ui(raw, |ui| {
                pad.show(ui);
                let response = ui.add(egui::TextEdit::singleline(value).id(field));
                if focus {
                    response.request_focus();
                }
            });
            output.textures_delta.clear();
        };
        frame(&mut pad, &mut value, Vec::new(), true);
        pad.key(egui::Key::A, egui::Modifiers { ctrl: true, command: true, ..Default::default() });
        frame(&mut pad, &mut value, Vec::new(), false);
        let pos = pad.key_rects.get("7").unwrap().center();
        for pressed in [true, false] {
            frame(
                &mut pad,
                &mut value,
                vec![
                    egui::Event::PointerMoved(pos),
                    egui::Event::PointerButton { pos, button: egui::PointerButton::Primary, pressed, modifiers: Default::default() },
                ],
                false,
            );
        }
        frame(&mut pad, &mut value, Vec::new(), false);
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
