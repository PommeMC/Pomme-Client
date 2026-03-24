use super::*;

impl MainMenu {
    pub(super) fn build_server_list(
        &mut self,
        screen_w: f32,
        screen_h: f32,
        input: &MenuInput,
        text_width_fn: &dyn Fn(&str, f32) -> f32,
    ) -> MainMenuResult {
        let gs = crate::ui::hud::gui_scale(screen_w, screen_h, self.gui_scale_setting);
        let header_h = HEADER_H * gs;
        let sep_h = SEP_H * gs;
        let entry_h = ENTRY_H * gs;
        let row_w = ROW_W * gs;
        let gap = BTN_GAP * gs;
        let fs = common::FONT_SIZE * gs;
        let btn_h = common::BTN_H * gs;
        let top_w = TOP_BTN_W * gs;
        let bot_w = BOT_BTN_W * gs;
        let cursor = input.cursor;
        let clicked = input.clicked;

        let footer_h = btn_h * 2.0 + gap * 3.0;
        let list_top = header_h + sep_h + gap;
        let list_bottom = screen_h - footer_h - sep_h;
        let list_h = list_bottom - list_top;

        let mut elements = Vec::new();
        let mut action = MenuAction::None;
        let mut any_hovered = false;

        if input.f5 {
            self.refresh_servers();
        }
        if input.escape {
            self.screen = Screen::Main;
            return MainMenuResult {
                elements: Vec::new(),
                action: MenuAction::None,
                cursor_pointer: false,
                blur: 1.0,
                clicked_button: false,
            };
        }

        elements.push(MenuElement::Text {
            x: screen_w / 2.0,
            y: (header_h - fs) / 2.0,
            text: "Multiplayer".into(),
            scale: fs,
            color: WHITE,
            centered: true,
        });

        push_separator(&mut elements, 0.0, header_h, screen_w, sep_h);
        push_separator(&mut elements, 0.0, list_bottom, screen_w, sep_h);

        let total_content = self.server_list.servers.len() as f32 * entry_h;
        let max_scroll = (total_content - list_h).max(0.0);
        self.scroll_offset =
            (self.scroll_offset - input.scroll_delta * entry_h).clamp(0.0, max_scroll);

        let list_cx = screen_w / 2.0;
        let list_left = list_cx - row_w / 2.0;
        let ping_results = self.ping_results.read().clone();

        for (i, server) in self.server_list.servers.iter().enumerate() {
            let ey = list_top + i as f32 * entry_h - self.scroll_offset;
            if ey + entry_h < list_top || ey > list_bottom {
                continue;
            }

            let selected = self.selected_server == Some(i);
            let rect = [list_left, ey, row_w, entry_h];
            let hovered =
                common::hit_test(cursor, rect) && cursor.1 >= list_top && cursor.1 <= list_bottom;
            any_hovered |= hovered;

            if selected || hovered {
                elements.push(MenuElement::Rect {
                    x: rect[0],
                    y: rect[1],
                    w: rect[2],
                    h: rect[3],
                    corner_radius: 0.0,
                    color: if selected {
                        [1.0, 1.0, 1.0, 0.12]
                    } else {
                        [1.0, 1.0, 1.0, 0.06]
                    },
                });
            }
            if selected {
                push_outline(&mut elements, rect[0], rect[1], rect[2], rect[3], gs);
            }

            let text_x = rect[0] + 3.0 * gs;
            let name_y = rect[1] + 1.0 * gs;
            elements.push(MenuElement::Text {
                x: text_x,
                y: name_y,
                text: server.name.clone(),
                scale: fs,
                color: WHITE,
                centered: false,
            });

            let motd_y = name_y + fs + 3.0 * gs;
            push_server_status(
                &mut elements,
                &ping_results,
                &server.address,
                text_x,
                motd_y,
                &rect,
                fs,
                gs,
                text_width_fn,
            );

            if clicked && hovered {
                let now = Instant::now();
                let is_double = self.last_click_index == Some(i)
                    && now.duration_since(self.last_click_time).as_millis() < DOUBLE_CLICK_MS;

                if is_double {
                    action = MenuAction::Connect {
                        server: server.address.clone(),
                        username: self.username.clone(),
                    };
                } else {
                    self.selected_server = Some(i);
                    self.last_click_time = now;
                    self.last_click_index = Some(i);
                }
            }
        }

        if self.server_list.servers.is_empty() {
            elements.push(MenuElement::Text {
                x: screen_w / 2.0,
                y: list_top + 40.0 * gs,
                text: "No servers added".into(),
                scale: fs,
                color: COL_DIM,
                centered: true,
            });
        }

        let has_sel = self.selected_server.is_some();
        let footer_y = list_bottom + sep_h + gap;

        let row1_w = top_w * 3.0 + gap * 2.0;
        let row1_x = (screen_w - row1_w) / 2.0;

        if push_button(
            &mut elements,
            &mut any_hovered,
            cursor,
            row1_x,
            footer_y,
            top_w,
            btn_h,
            gs,
            "Join Server",
            has_sel,
        ) && clicked
        {
            if let Some(idx) = self.selected_server {
                if let Some(server) = self.server_list.servers.get(idx) {
                    action = MenuAction::Connect {
                        server: server.address.clone(),
                        username: self.username.clone(),
                    };
                }
            }
        }
        if push_button(
            &mut elements,
            &mut any_hovered,
            cursor,
            row1_x + top_w + gap,
            footer_y,
            top_w,
            btn_h,
            gs,
            "Direct Connect",
            true,
        ) && clicked
        {
            self.edit_address = self.last_mp_ip.clone();
            self.focused_field = Some(0);
            self.cursor_blink = Instant::now();
            self.screen = Screen::DirectConnect;
        }
        if push_button(
            &mut elements,
            &mut any_hovered,
            cursor,
            row1_x + (top_w + gap) * 2.0,
            footer_y,
            top_w,
            btn_h,
            gs,
            "Add Server",
            true,
        ) && clicked
        {
            self.edit_name.clear();
            self.edit_address.clear();
            self.focused_field = Some(0);
            self.cursor_blink = Instant::now();
            self.screen = Screen::AddServer;
        }

        let row2_y = footer_y + btn_h + gap;
        let row2_w = bot_w * 4.0 + gap * 3.0;
        let row2_x = (screen_w - row2_w) / 2.0;

        if push_button(
            &mut elements,
            &mut any_hovered,
            cursor,
            row2_x,
            row2_y,
            bot_w,
            btn_h,
            gs,
            "Edit",
            has_sel,
        ) && clicked
        {
            if let Some(idx) = self.selected_server {
                if let Some(server) = self.server_list.servers.get(idx) {
                    self.edit_name = server.name.clone();
                    self.edit_address = server.address.clone();
                    self.focused_field = Some(0);
                    self.cursor_blink = Instant::now();
                    self.screen = Screen::EditServer(idx);
                }
            }
        }
        if push_button(
            &mut elements,
            &mut any_hovered,
            cursor,
            row2_x + bot_w + gap,
            row2_y,
            bot_w,
            btn_h,
            gs,
            "Delete",
            has_sel,
        ) && clicked
        {
            if let Some(idx) = self.selected_server {
                self.screen = Screen::ConfirmDelete(idx);
            }
        }
        if push_button(
            &mut elements,
            &mut any_hovered,
            cursor,
            row2_x + (bot_w + gap) * 2.0,
            row2_y,
            bot_w,
            btn_h,
            gs,
            "Refresh",
            true,
        ) && clicked
        {
            self.refresh_servers();
        }
        if push_button(
            &mut elements,
            &mut any_hovered,
            cursor,
            row2_x + (bot_w + gap) * 3.0,
            row2_y,
            bot_w,
            btn_h,
            gs,
            "Back",
            true,
        ) && clicked
        {
            self.screen = Screen::Main;
        }

        push_bottom_text(&mut elements, screen_w, screen_h, gs, text_width_fn);
        MainMenuResult {
            elements,
            action,
            cursor_pointer: any_hovered,
            blur: 2.0,
            clicked_button: false,
        }
    }

    pub(super) fn build_confirm_delete(
        &mut self,
        screen_w: f32,
        screen_h: f32,
        input: &MenuInput,
        text_width_fn: &dyn Fn(&str, f32) -> f32,
    ) -> MainMenuResult {
        let Screen::ConfirmDelete(idx) = self.screen else {
            return empty_result(2.0);
        };

        let gs = crate::ui::hud::gui_scale(screen_w, screen_h, self.gui_scale_setting);
        let fs = common::FONT_SIZE * gs;
        let form_w = FORM_W * gs;
        let btn_h = common::BTN_H * gs;
        let gap = BTN_GAP * gs;
        let cursor = input.cursor;
        let clicked = input.clicked;

        if input.escape {
            self.screen = Screen::ServerList;
            return empty_result(2.0);
        }

        let warning = self
            .server_list
            .servers
            .get(idx)
            .map(|s| format!("'{}' will be lost forever! (A long time!)", s.name))
            .unwrap_or_default();

        let mut elements = Vec::new();
        let mut any_hovered = false;

        let cy = screen_h * 0.3;
        elements.push(MenuElement::Text {
            x: screen_w / 2.0,
            y: cy,
            text: "Are you sure?".into(),
            scale: fs,
            color: WHITE,
            centered: true,
        });
        elements.push(MenuElement::Text {
            x: screen_w / 2.0,
            y: cy + fs + 12.0 * gs,
            text: warning,
            scale: fs,
            color: COL_DIM,
            centered: true,
        });

        let btn_x = (screen_w - form_w) / 2.0;
        let btn_y = cy + fs * 2.0 + 44.0 * gs;

        if push_button(
            &mut elements,
            &mut any_hovered,
            cursor,
            btn_x,
            btn_y,
            form_w,
            btn_h,
            gs,
            "Delete",
            true,
        ) && clicked
        {
            self.server_list.remove(idx);
            self.selected_server = None;
            self.screen = Screen::ServerList;
        }
        if push_button(
            &mut elements,
            &mut any_hovered,
            cursor,
            btn_x,
            btn_y + btn_h + gap,
            form_w,
            btn_h,
            gs,
            "Cancel",
            true,
        ) && clicked
        {
            self.screen = Screen::ServerList;
        }

        push_bottom_text(&mut elements, screen_w, screen_h, gs, text_width_fn);
        MainMenuResult {
            elements,
            action: MenuAction::None,
            cursor_pointer: any_hovered,
            blur: 2.0,
            clicked_button: false,
        }
    }

    pub(super) fn build_direct_connect(
        &mut self,
        screen_w: f32,
        screen_h: f32,
        input: &MenuInput,
        text_width_fn: &dyn Fn(&str, f32) -> f32,
    ) -> MainMenuResult {
        let gs = crate::ui::hud::gui_scale(screen_w, screen_h, self.gui_scale_setting);
        let fs = common::FONT_SIZE * gs;
        let form_w = FORM_W * gs;
        let btn_h = common::BTN_H * gs;
        let gap = BTN_GAP * gs;
        let field_h = FIELD_H * gs;
        let cursor = input.cursor;
        let clicked = input.clicked;

        if input.escape {
            self.screen = Screen::ServerList;
            return empty_result(2.0);
        }

        self.handle_text_input(input, 1);

        let mut elements = Vec::new();
        let mut action = MenuAction::None;
        let mut any_hovered = false;

        let cx = screen_w / 2.0;
        let form_x = cx - form_w / 2.0;
        let mut y = 20.0 * gs;

        elements.push(MenuElement::Text {
            x: cx,
            y,
            text: "Direct Connect".into(),
            scale: fs,
            color: WHITE,
            centered: true,
        });
        y += fs + 40.0 * gs;

        elements.push(MenuElement::Text {
            x: form_x,
            y,
            text: "Server Address".into(),
            scale: fs,
            color: COL_DIM,
            centered: false,
        });
        y += fs + 4.0 * gs;

        push_text_field(
            &mut elements,
            form_x,
            y,
            form_w,
            field_h,
            fs,
            gs,
            &self.edit_address,
            self.focused_field == Some(0),
            &self.cursor_blink,
            text_width_fn,
        );
        if clicked && common::hit_test(cursor, [form_x, y, form_w, field_h]) {
            self.focused_field = Some(0);
            self.cursor_blink = Instant::now();
        }
        y += field_h + 28.0 * gs;

        let valid = is_valid_address(&self.edit_address);
        let enter_submit = input.enter && valid;

        if (push_button(
            &mut elements,
            &mut any_hovered,
            cursor,
            form_x,
            y,
            form_w,
            btn_h,
            gs,
            "Join Server",
            valid,
        ) && clicked)
            || enter_submit
        {
            self.last_mp_ip = self.edit_address.clone();
            action = MenuAction::Connect {
                server: self.edit_address.clone(),
                username: self.username.clone(),
            };
        }
        y += btn_h + gap;
        if push_button(
            &mut elements,
            &mut any_hovered,
            cursor,
            form_x,
            y,
            form_w,
            btn_h,
            gs,
            "Cancel",
            true,
        ) && clicked
        {
            self.screen = Screen::ServerList;
        }

        push_bottom_text(&mut elements, screen_w, screen_h, gs, text_width_fn);
        MainMenuResult {
            elements,
            action,
            cursor_pointer: any_hovered,
            blur: 2.0,
            clicked_button: false,
        }
    }

    pub(super) fn build_edit_server(
        &mut self,
        screen_w: f32,
        screen_h: f32,
        input: &MenuInput,
        text_width_fn: &dyn Fn(&str, f32) -> f32,
    ) -> MainMenuResult {
        let gs = crate::ui::hud::gui_scale(screen_w, screen_h, self.gui_scale_setting);
        let fs = common::FONT_SIZE * gs;
        let form_w = FORM_W * gs;
        let btn_h = common::BTN_H * gs;
        let gap = BTN_GAP * gs;
        let field_h = FIELD_H * gs;
        let cursor = input.cursor;
        let clicked = input.clicked;

        if input.escape {
            self.screen = Screen::ServerList;
            return empty_result(2.0);
        }

        self.handle_text_input(input, 2);

        let mut elements = Vec::new();
        let mut any_hovered = false;

        let cx = screen_w / 2.0;
        let form_x = cx - form_w / 2.0;
        let mut y = 17.0 * gs;

        elements.push(MenuElement::Text {
            x: cx,
            y,
            text: "Edit Server Info".into(),
            scale: fs,
            color: WHITE,
            centered: true,
        });
        y += fs + 20.0 * gs;

        elements.push(MenuElement::Text {
            x: form_x,
            y,
            text: "Server Name".into(),
            scale: fs,
            color: COL_DIM,
            centered: false,
        });
        y += fs + 4.0 * gs;

        push_text_field(
            &mut elements,
            form_x,
            y,
            form_w,
            field_h,
            fs,
            gs,
            &self.edit_name,
            self.focused_field == Some(0),
            &self.cursor_blink,
            text_width_fn,
        );
        if clicked && common::hit_test(cursor, [form_x, y, form_w, field_h]) {
            self.focused_field = Some(0);
            self.cursor_blink = Instant::now();
        }
        y += field_h + 12.0 * gs;

        elements.push(MenuElement::Text {
            x: form_x,
            y,
            text: "Server Address".into(),
            scale: fs,
            color: COL_DIM,
            centered: false,
        });
        y += fs + 4.0 * gs;

        push_text_field(
            &mut elements,
            form_x,
            y,
            form_w,
            field_h,
            fs,
            gs,
            &self.edit_address,
            self.focused_field == Some(1),
            &self.cursor_blink,
            text_width_fn,
        );
        if clicked && common::hit_test(cursor, [form_x, y, form_w, field_h]) {
            self.focused_field = Some(1);
            self.cursor_blink = Instant::now();
        }
        y += field_h + 28.0 * gs;

        let valid = is_valid_address(&self.edit_address);
        if push_button(
            &mut elements,
            &mut any_hovered,
            cursor,
            form_x,
            y,
            form_w,
            btn_h,
            gs,
            "Done",
            valid,
        ) && clicked
        {
            let name = if self.edit_name.is_empty() {
                "Minecraft Server".to_string()
            } else {
                self.edit_name.clone()
            };
            let addr = self.edit_address.clone();
            let entry = ServerEntry {
                name,
                address: addr.clone(),
            };
            if let Screen::EditServer(idx) = self.screen {
                self.server_list.update(idx, entry);
            } else {
                self.server_list.add(entry);
            }
            ping_all_servers(
                &self.rt,
                &[ServerEntry {
                    name: String::new(),
                    address: addr,
                }],
                &self.ping_results,
            );
            self.screen = Screen::ServerList;
        }
        y += btn_h + gap;
        if push_button(
            &mut elements,
            &mut any_hovered,
            cursor,
            form_x,
            y,
            form_w,
            btn_h,
            gs,
            "Cancel",
            true,
        ) && clicked
        {
            self.screen = Screen::ServerList;
        }

        push_bottom_text(&mut elements, screen_w, screen_h, gs, text_width_fn);
        MainMenuResult {
            elements,
            action: MenuAction::None,
            cursor_pointer: any_hovered,
            blur: 2.0,
            clicked_button: false,
        }
    }

    pub(super) fn handle_text_input(&mut self, input: &MenuInput, field_count: u8) {
        if input.tab {
            self.focused_field = Some(match self.focused_field {
                Some(f) => (f + 1) % field_count,
                None => 0,
            });
            self.cursor_blink = Instant::now();
        }

        let Some(field_idx) = self.focused_field else {
            return;
        };
        let is_edit_form = matches!(self.screen, Screen::AddServer | Screen::EditServer(_));
        let text = match (is_edit_form, field_idx) {
            (true, 0) => &mut self.edit_name,
            (true, 1) => &mut self.edit_address,
            (false, 0) => &mut self.edit_address,
            _ => return,
        };

        for ch in &input.typed_chars {
            text.push(*ch);
        }
        if input.backspace {
            text.pop();
        }
        if !input.typed_chars.is_empty() || input.backspace {
            self.cursor_blink = Instant::now();
        }
    }

    pub(super) fn build_disconnected(
        &mut self,
        screen_w: f32,
        screen_h: f32,
        input: &MenuInput,
        _text_width_fn: &dyn Fn(&str, f32) -> f32,
    ) -> MainMenuResult {
        let reason = match &self.screen {
            Screen::Disconnected(r) => r.clone(),
            _ => return empty_result(2.0),
        };

        let gs = crate::ui::hud::gui_scale(screen_w, screen_h, self.gui_scale_setting);
        let title_size = 18.0 * gs;
        let body_size = 11.0 * gs;
        let btn_w = 160.0 * gs;
        let btn_h = 30.0 * gs;
        let gap = 12.0 * gs;

        let cx = screen_w / 2.0;
        let total_h = title_size + gap + body_size + gap * 2.0 + btn_h;
        let top_y = (screen_h - total_h) / 2.0;

        let mut elements = Vec::new();
        let mut any_hovered = false;

        elements.push(MenuElement::Text {
            x: cx,
            y: top_y,
            text: "Disconnected".into(),
            scale: title_size,
            color: [1.0, 0.4, 0.4, 1.0],
            centered: true,
        });

        elements.push(MenuElement::Text {
            x: cx,
            y: top_y + title_size + gap,
            text: reason,
            scale: body_size,
            color: [0.85, 0.85, 0.85, 0.9],
            centered: true,
        });

        let btn_y = top_y + title_size + gap + body_size + gap * 2.0;
        if push_button(
            &mut elements,
            &mut any_hovered,
            input.cursor,
            cx - btn_w / 2.0,
            btn_y,
            btn_w,
            btn_h,
            gs,
            "Back to Menu",
            true,
        ) && input.clicked
        {
            self.screen = Screen::Main;
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
