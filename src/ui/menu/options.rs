use super::*;

impl MainMenu {
    pub(super) fn build_options(&mut self, sw: f32, sh: f32, input: &MenuInput) -> MainMenuResult {
        let fov_label = format!("FOV: {}", 70);
        let rows: Vec<[&str; 2]> = vec![
            [&fov_label, "Online"],
            ["Skin Customization...", "Music & Sounds..."],
            ["Video Settings...", "Controls..."],
            ["Language...", "Chat Settings..."],
            ["Resource Packs...", "Accessibility Settings..."],
            ["Telemetry Data...", "Credits & Attribution..."],
        ];

        let nav: &[(&str, Screen)] = &[
            ("Skin Customization...", Screen::OptionsSkinCustomization),
            ("Music & Sounds...", Screen::OptionsMusicSounds),
            ("Video Settings...", Screen::OptionsVideo),
            ("Controls...", Screen::OptionsControls),
            ("Language...", Screen::OptionsLanguage),
            ("Chat Settings...", Screen::OptionsChatSettings),
            ("Resource Packs...", Screen::OptionsResourcePacks),
            ("Accessibility Settings...", Screen::OptionsAccessibility),
            ("Telemetry Data...", Screen::OptionsTelemetry),
            ("Credits & Attribution...", Screen::OptionsCredits),
        ];

        self.build_options_grid(sw, sh, input, "Options", Screen::Main, &rows, nav, &[])
    }

    pub(super) fn build_options_video(
        &mut self,
        sw: f32,
        sh: f32,
        input: &MenuInput,
    ) -> MainMenuResult {
        let fullscreen_label = match self.display_mode {
            DisplayMode::Windowed => "Fullscreen: Windowed",
            DisplayMode::Borderless => "Fullscreen: Borderless",
            DisplayMode::Fullscreen => "Fullscreen: Exclusive",
        };
        let rd = format!("Render Distance: {} chunks", self.render_distance);
        let sd = format!("Simulation Distance: {} chunks", self.render_distance);
        let mf = format!("Max Framerate: {} fps", 120);
        let gui_label = if self.gui_scale_setting == 0 {
            "GUI Scale: Auto".to_string()
        } else {
            format!("GUI Scale: {}", self.gui_scale_setting)
        };
        let rows: Vec<[&str; 2]> = vec![
            [&rd, &sd],
            ["Graphics: Fancy", "Smooth Lighting: ON"],
            [&mf, "VSync: OFF"],
            ["View Bobbing: ON", &gui_label],
            ["Attack Indicator: Crosshair", "Brightness: 50%"],
            ["Clouds: Fancy", fullscreen_label],
            ["Particles: All", "Mipmap Levels: 4"],
        ];
        let rd_frac = (self.render_distance as f32 - 2.0) / 30.0;
        let sd_frac = (self.simulation_distance as f32 - 5.0) / 27.0;
        let sliders: &[(&str, f32)] = &[
            ("Render Distance:", rd_frac),
            ("Simulation Distance:", sd_frac),
        ];
        self.build_options_grid(
            sw,
            sh,
            input,
            "Video Settings",
            Screen::Options,
            &rows,
            &[],
            sliders,
        )
    }

    pub(super) fn build_options_controls(
        &mut self,
        sw: f32,
        sh: f32,
        input: &MenuInput,
    ) -> MainMenuResult {
        let rows: Vec<[&str; 2]> = vec![
            ["Sensitivity: 100%", "Invert Mouse: OFF"],
            ["Auto-Jump: ON", "Operator Items Tab: OFF"],
            ["Key Binds...", "Mouse Settings..."],
            ["Sneak: Toggle", "Sprint: Hold"],
        ];
        let nav: &[(&str, Screen)] = &[("Key Binds...", Screen::OptionsKeybinds)];
        self.build_options_grid(sw, sh, input, "Controls", Screen::Options, &rows, nav, &[])
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn build_options_grid(
        &mut self,
        sw: f32,
        sh: f32,
        input: &MenuInput,
        title: &str,
        back: Screen,
        rows: &[[&str; 2]],
        nav: &[(&str, Screen)],
        sliders: &[(&'static str, f32)],
    ) -> MainMenuResult {
        if input.escape {
            self.screen = back.clone_screen();
            return empty_result(2.0);
        }

        let gs = crate::ui::hud::gui_scale(sw, sh, self.gui_scale_setting);
        let fs = common::FONT_SIZE * gs;
        let btn_h = common::BTN_H * gs;
        let gap = BTN_GAP * gs;
        let header_h = HEADER_H * gs;
        let sep_h = SEP_H * gs;
        let btn_w = 150.0 * gs;
        let half_w = (btn_w * 2.0 + gap) / 2.0;
        let cx = sw / 2.0;
        let cursor = input.cursor;
        let clicked = input.clicked;

        let mut elements = Vec::new();
        let mut any_hovered = false;

        common::push_overlay(&mut elements, sw, sh, 0.5);

        elements.push(MenuElement::Text {
            x: cx,
            y: (header_h - fs) / 2.0,
            text: title.into(),
            scale: fs,
            color: WHITE,
            centered: true,
        });
        push_separator(&mut elements, 0.0, header_h, sw, sep_h);

        let done_pad = 8.0 * gs;
        let done_y = sh - btn_h - done_pad;
        let content_top = header_h + sep_h;
        let content_bottom = done_y;
        let grid_h = rows.len() as f32 * btn_h + (rows.len() as f32 - 1.0).max(0.0) * gap;
        let top_y = content_top + (content_bottom - content_top - grid_h) / 2.0;
        let lx = cx - half_w;
        let rx = lx + btn_w + gap;

        let mut slider_results: Vec<(&str, f32)> = Vec::new();

        for (row, pair) in rows.iter().enumerate() {
            let by = top_y + row as f32 * (btn_h + gap);
            for (col, label) in pair.iter().enumerate() {
                let bx = if col == 0 { lx } else { rx };

                if let Some((prefix, value)) = sliders.iter().find(|(p, _)| label.starts_with(p)) {
                    let is_active = self.active_slider == Some(*prefix);
                    let result = common::push_slider(
                        &mut elements,
                        cursor,
                        input.mouse_held,
                        bx,
                        by,
                        btn_w,
                        btn_h,
                        gs,
                        fs,
                        label,
                        *value,
                        is_active,
                    );
                    any_hovered |= result.hovered;
                    if result.dragging {
                        self.active_slider = Some(*prefix);
                    }
                    if let Some(v) = result.new_value {
                        slider_results.push((prefix, v));
                    }
                    if !input.mouse_held && is_active {
                        self.active_slider = None;
                    }
                    continue;
                }

                let h = common::push_button(
                    &mut elements,
                    cursor,
                    bx,
                    by,
                    btn_w,
                    btn_h,
                    gs,
                    fs,
                    label,
                    true,
                );
                any_hovered |= h;
                if clicked && h {
                    if let Some((_, target)) = nav.iter().find(|(l, _)| *l == *label) {
                        self.screen = target.clone_screen();
                    }
                    if label.starts_with("GUI Scale:") {
                        let max = crate::ui::hud::max_gui_scale(sw, sh);
                        self.gui_scale_setting = (self.gui_scale_setting + 1) % (max + 1);
                        self.save_settings();
                    }
                    if label.starts_with("Fullscreen:") {
                        self.display_mode = self.display_mode.cycle();
                    }
                }
            }
        }

        for (prefix, value) in &slider_results {
            if *prefix == "Render Distance:" {
                self.render_distance = (2.0 + value * 30.0).round() as u32;
                self.save_settings();
            }
            if *prefix == "Simulation Distance:" {
                self.simulation_distance = (5.0 + value * 27.0).round() as u32;
                self.save_settings();
            }
        }

        let done_w = btn_w * 2.0 + gap;
        let h = common::push_button(
            &mut elements,
            cursor,
            cx - done_w / 2.0,
            done_y,
            done_w,
            btn_h,
            gs,
            fs,
            "Done",
            true,
        );
        any_hovered |= h;
        if clicked && h {
            self.screen = back;
        }

        MainMenuResult {
            elements,
            action: MenuAction::None,
            cursor_pointer: any_hovered,
            blur: 2.0,
            clicked_button: false,
        }
    }

    pub(super) fn build_options_stub(
        &mut self,
        sw: f32,
        sh: f32,
        input: &MenuInput,
        title: &str,
        back: Screen,
    ) -> MainMenuResult {
        if input.escape {
            self.screen = back.clone_screen();
            return empty_result(2.0);
        }

        let gs = crate::ui::hud::gui_scale(sw, sh, self.gui_scale_setting);
        let fs = common::FONT_SIZE * gs;
        let btn_h = common::BTN_H * gs;
        let gap = BTN_GAP * gs;
        let header_h = HEADER_H * gs;
        let sep_h = SEP_H * gs;
        let cx = sw / 2.0;

        let mut elements = Vec::new();
        let mut any_hovered = false;

        common::push_overlay(&mut elements, sw, sh, 0.5);

        elements.push(MenuElement::Text {
            x: cx,
            y: (header_h - fs) / 2.0,
            text: title.into(),
            scale: fs,
            color: WHITE,
            centered: true,
        });
        push_separator(&mut elements, 0.0, header_h, sw, sep_h);

        let body_fs = 10.0 * gs;
        elements.push(MenuElement::Text {
            x: cx,
            y: sh / 2.0 - body_fs,
            text: "Coming soon".into(),
            scale: body_fs,
            color: COL_DIM,
            centered: true,
        });

        let done_w = 150.0 * gs * 2.0 + gap;
        let done_y = sh - btn_h - 8.0 * gs;
        let h = common::push_button(
            &mut elements,
            input.cursor,
            cx - done_w / 2.0,
            done_y,
            done_w,
            btn_h,
            gs,
            fs,
            "Done",
            true,
        );
        any_hovered |= h;
        if input.clicked && h {
            self.screen = back;
        }

        MainMenuResult {
            elements,
            action: MenuAction::None,
            cursor_pointer: any_hovered,
            blur: 2.0,
            clicked_button: false,
        }
    }
}
