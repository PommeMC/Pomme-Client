use crate::renderer::pipelines::menu_overlay::MenuElement;

pub const WHITE: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
pub const FONT_SIZE: f32 = 8.0;
pub const BTN_H: f32 = 20.0;
pub const BTN_NORMAL: [f32; 4] = [0.3, 0.3, 0.3, 0.8];
pub const BTN_HOVER: [f32; 4] = [0.45, 0.45, 0.55, 0.9];
pub const BTN_DISABLED: [f32; 4] = [0.12, 0.12, 0.12, 0.7];
pub const COL_DISABLED: [f32; 4] = [0.4, 0.4, 0.4, 1.0];

pub fn push_overlay(elements: &mut Vec<MenuElement>, screen_w: f32, screen_h: f32, alpha: f32) {
    elements.push(MenuElement::Rect {
        x: 0.0,
        y: 0.0,
        w: screen_w,
        h: screen_h,
        corner_radius: 0.0,
        color: [0.0, 0.0, 0.0, alpha],
    });
}

pub fn hit_test(cursor: (f32, f32), rect: [f32; 4]) -> bool {
    cursor.0 >= rect[0]
        && cursor.0 <= rect[0] + rect[2]
        && cursor.1 >= rect[1]
        && cursor.1 <= rect[1] + rect[3]
}

#[allow(clippy::too_many_arguments)]
pub fn push_button(
    elements: &mut Vec<MenuElement>,
    cursor: (f32, f32),
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    gs: f32,
    fs: f32,
    label: &str,
    enabled: bool,
) -> bool {
    let hovered = enabled && hit_test(cursor, [x, y, w, h]);

    let (bg, text_col) = if !enabled {
        (BTN_DISABLED, COL_DISABLED)
    } else if hovered {
        (BTN_HOVER, WHITE)
    } else {
        (BTN_NORMAL, WHITE)
    };

    elements.push(MenuElement::Rect {
        x,
        y,
        w,
        h,
        corner_radius: 2.0 * gs,
        color: bg,
    });
    elements.push(MenuElement::Text {
        x: x + w / 2.0,
        y: y + (h - fs) / 2.0,
        text: label.into(),
        scale: fs,
        color: text_col,
        centered: true,
    });

    hovered
}

const SLIDER_TRACK: [f32; 4] = [0.0, 0.0, 0.0, 0.5];
const SLIDER_KNOB: [f32; 4] = [0.8, 0.8, 0.8, 1.0];
const SLIDER_KNOB_HOVER: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
const SLIDER_KNOB_DRAG: [f32; 4] = [0.6, 0.75, 1.0, 1.0];

#[allow(clippy::too_many_arguments)]
pub fn push_slider(
    elements: &mut Vec<MenuElement>,
    cursor: (f32, f32),
    mouse_held: bool,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    gs: f32,
    fs: f32,
    label: &str,
    value: f32,
    dragging: bool,
) -> SliderResult {
    let hovered = hit_test(cursor, [x, y, w, h]);
    let knob_w = 8.0 * gs;
    let track_pad = 2.0 * gs;
    let track_w = w - track_pad * 2.0;
    let knob_x = x + track_pad + value.clamp(0.0, 1.0) * (track_w - knob_w);

    let actively_dragging = dragging && mouse_held;
    let start_drag = hovered && mouse_held && !dragging;

    let new_value = if actively_dragging || start_drag {
        let rel = (cursor.0 - x - track_pad - knob_w / 2.0) / (track_w - knob_w);
        Some(rel.clamp(0.0, 1.0))
    } else {
        None
    };

    elements.push(MenuElement::Rect {
        x,
        y,
        w,
        h,
        corner_radius: 2.0 * gs,
        color: SLIDER_TRACK,
    });

    let fill_w = knob_x - x + knob_w / 2.0;
    elements.push(MenuElement::Rect {
        x,
        y,
        w: fill_w,
        h,
        corner_radius: 2.0 * gs,
        color: BTN_NORMAL,
    });

    let knob_color = if actively_dragging || start_drag {
        SLIDER_KNOB_DRAG
    } else if hovered {
        SLIDER_KNOB_HOVER
    } else {
        SLIDER_KNOB
    };
    elements.push(MenuElement::Rect {
        x: knob_x,
        y: y + 1.0 * gs,
        w: knob_w,
        h: h - 2.0 * gs,
        corner_radius: 2.0 * gs,
        color: knob_color,
    });

    elements.push(MenuElement::Text {
        x: x + w / 2.0,
        y: y + (h - fs) / 2.0,
        text: label.into(),
        scale: fs,
        color: WHITE,
        centered: true,
    });

    SliderResult {
        hovered,
        dragging: actively_dragging || start_drag,
        new_value,
    }
}

pub struct SliderResult {
    pub hovered: bool,
    pub dragging: bool,
    pub new_value: Option<f32>,
}

pub fn push_cursor_blink(
    elements: &mut Vec<MenuElement>,
    cursor_blink: &std::time::Instant,
    x: f32,
    y: f32,
    gs: f32,
    fs: f32,
    text_width: f32,
) {
    if cursor_blink.elapsed().as_millis() % 1000 < 500 {
        elements.push(MenuElement::Rect {
            x: x + text_width,
            y,
            w: 1.0 * gs,
            h: fs,
            corner_radius: 0.0,
            color: WHITE,
        });
    }
}
