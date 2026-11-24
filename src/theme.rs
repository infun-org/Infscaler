//! Windows 11 / Fluent 2 light theme for the InfScaler GUI.

use eframe::egui::{
    self, style::WidgetVisuals, Color32, CornerRadius, FontData, FontDefinitions, FontFamily,
    FontId, Frame, Margin, RichText, Shadow, Stroke, TextStyle, Theme, ThemePreference, Vec2,
    Visuals,
};

pub const MICA: Color32 = Color32::from_rgb(0xF3, 0xF3, 0xF3);
pub const CARD: Color32 = Color32::from_rgb(0xFB, 0xFB, 0xFB);
pub const WHITE: Color32 = Color32::from_rgb(0xFF, 0xFF, 0xFF);
pub const BORDER: Color32 = Color32::from_rgb(0xE5, 0xE5, 0xE5);
pub const BORDER_DARK: Color32 = Color32::from_rgb(0xD0, 0xD0, 0xD0);
pub const TEXT: Color32 = Color32::from_rgb(0x1B, 0x1B, 0x1B);
pub const TEXT_SECONDARY: Color32 = Color32::from_rgb(0x5F, 0x5F, 0x5F);
pub const TEXT_DISABLED: Color32 = Color32::from_rgb(0xA0, 0xA0, 0xA0);
pub const ACCENT: Color32 = Color32::from_rgb(0x00, 0x5F, 0xB8);
pub const ACCENT_HOVER: Color32 = Color32::from_rgb(0x00, 0x67, 0xC0);
pub const ACCENT_PRESSED: Color32 = Color32::from_rgb(0x00, 0x4C, 0x94);
pub const ACCENT_TINT: Color32 = Color32::from_rgb(0xE8, 0xF1, 0xFA);
pub const HOVER: Color32 = Color32::from_rgb(0xF5, 0xF5, 0xF5);
pub const ERROR: Color32 = Color32::from_rgb(0xC4, 0x2B, 0x1C);
pub const SUCCESS: Color32 = Color32::from_rgb(0x0F, 0x7B, 0x0F);
pub const CAUTION: Color32 = Color32::from_rgb(0x9D, 0x5D, 0x00);

pub const RADIUS_CONTROL: u8 = 4;
pub const RADIUS_CARD: u8 = 8;

pub const SIZE_BODY: f32 = 14.0;
pub const SIZE_CAPTION: f32 = 12.0;
pub const SIZE_SUBTITLE: f32 = 20.0;
pub const SIZE_TITLE: f32 = 28.0;

pub fn family_medium() -> FontFamily {
    FontFamily::Name("medium".into())
}

pub fn title_text(text: impl Into<String>) -> RichText {
    RichText::new(text)
        .font(FontId::new(SIZE_TITLE, family_medium()))
        .color(TEXT)
}

pub fn subtitle_text(text: impl Into<String>) -> RichText {
    RichText::new(text)
        .font(FontId::new(SIZE_SUBTITLE, family_medium()))
        .color(TEXT)
}

pub fn caption_text(text: impl Into<String>) -> RichText {
    RichText::new(text).size(SIZE_CAPTION).color(TEXT_SECONDARY)
}

pub fn card_frame() -> Frame {
    Frame::new()
        .fill(CARD)
        .stroke(Stroke::new(1.0, BORDER))
        .corner_radius(RADIUS_CARD)
        .inner_margin(8.0)
        .shadow(Shadow {
            offset: [0, 2],
            blur: 8,
            spread: 0,
            color: Color32::from_black_alpha(20),
        })
}

pub fn mica_panel() -> Frame {
    Frame::new()
        .fill(MICA)
        .inner_margin(Margin::symmetric(12, 12))
        .stroke(Stroke::NONE)
}

pub fn status_panel() -> Frame {
    Frame::new()
        .fill(MICA)
        .inner_margin(Margin::symmetric(16, 10))
        .stroke(Stroke::new(1.0, BORDER))
}

pub fn header_panel() -> Frame {
    Frame::new()
        .fill(MICA)
        .inner_margin(Margin::symmetric(16, 12))
        .stroke(Stroke::NONE)
}

fn widget(bg: Color32, stroke: Color32, fg: Color32, radius: u8) -> WidgetVisuals {
    WidgetVisuals {
        bg_fill: bg,
        weak_bg_fill: bg,
        bg_stroke: Stroke::new(1.0, stroke),
        fg_stroke: Stroke::new(1.0, fg),
        corner_radius: CornerRadius::same(radius),
        expansion: 0.0,
    }
}

pub fn apply(ctx: &egui::Context) {
    ctx.set_theme(ThemePreference::Light);
    install_fonts(ctx);

    let mut visuals = Visuals::light();
    visuals.dark_mode = false;
    visuals.window_fill = MICA;
    visuals.panel_fill = MICA;
    visuals.faint_bg_color = Color32::from_rgb(0xEE, 0xEE, 0xEE);
    visuals.extreme_bg_color = WHITE;
    visuals.code_bg_color = Color32::from_rgb(0xF5, 0xF5, 0xF5);
    visuals.hyperlink_color = ACCENT;
    visuals.warn_fg_color = CAUTION;
    visuals.error_fg_color = ERROR;
    visuals.window_corner_radius = CornerRadius::same(RADIUS_CARD);
    visuals.menu_corner_radius = CornerRadius::same(RADIUS_CARD);
    visuals.window_stroke = Stroke::new(1.0, BORDER);
    visuals.window_shadow = Shadow {
        offset: [0, 8],
        blur: 24,
        spread: 0,
        color: Color32::from_black_alpha(28),
    };
    visuals.popup_shadow = Shadow {
        offset: [0, 4],
        blur: 16,
        spread: 0,
        color: Color32::from_black_alpha(30),
    };
    visuals.selection.bg_fill = ACCENT;
    visuals.selection.stroke = Stroke::new(1.0, WHITE);
    visuals.weak_text_color = Some(TEXT_DISABLED);
    let _ = (ACCENT_HOVER, ACCENT_PRESSED, ACCENT_TINT, SUCCESS);
    visuals.text_cursor.stroke = Stroke::new(2.0, ACCENT);
    visuals.widgets.noninteractive = widget(MICA, BORDER, TEXT, RADIUS_CONTROL);
    visuals.widgets.inactive = widget(WHITE, BORDER, TEXT, RADIUS_CONTROL);
    visuals.widgets.hovered = widget(HOVER, BORDER_DARK, TEXT, RADIUS_CONTROL);
    visuals.widgets.active = widget(
        Color32::from_rgb(0xEB, 0xEB, 0xEB),
        ACCENT,
        TEXT,
        RADIUS_CONTROL,
    );
    visuals.widgets.open = widget(WHITE, ACCENT, TEXT, RADIUS_CONTROL);

    ctx.set_visuals_of(egui::Theme::Light, visuals.clone());
    ctx.set_visuals(visuals.clone());

    let mut style = (*ctx.style_of(Theme::Light)).clone();
    style.visuals = visuals.clone();
    style.spacing.item_spacing = Vec2::new(8.0, 8.0);
    style.spacing.button_padding = Vec2::new(12.0, 6.0);
    style.spacing.interact_size = Vec2::new(40.0, 32.0);
    style.spacing.icon_width = 16.0;
    style.spacing.icon_width_inner = 10.0;
    style.spacing.icon_spacing = 8.0;
    style.spacing.combo_width = 200.0;
    style.spacing.scroll.bar_width = 6.0;
    style.spacing.scroll.bar_inner_margin = 2.0;
    style.spacing.scroll.bar_outer_margin = 2.0;
    style.spacing.slider_rail_height = 4.0;
    style.text_styles.insert(
        TextStyle::Small,
        FontId::new(SIZE_CAPTION, FontFamily::Proportional),
    );
    style.text_styles.insert(
        TextStyle::Body,
        FontId::new(SIZE_BODY, FontFamily::Proportional),
    );
    style.text_styles.insert(
        TextStyle::Button,
        FontId::new(SIZE_BODY, FontFamily::Proportional),
    );
    style.text_styles.insert(
        TextStyle::Monospace,
        FontId::new(SIZE_CAPTION, FontFamily::Monospace),
    );
    style
        .text_styles
        .insert(TextStyle::Heading, FontId::new(SIZE_TITLE, family_medium()));
    ctx.set_style_of(egui::Theme::Light, style);
}

pub fn install_fonts(ctx: &egui::Context) {
    const REGULAR: &[&str] = &[
        "/usr/share/fonts/inter/Inter-Regular.ttf",
        "/usr/share/fonts/TTF/Inter-Regular.ttf",
        "/usr/share/fonts/truetype/inter/Inter-Regular.ttf",
        "/usr/share/fonts/noto/NotoSans-Regular.ttf",
        "/usr/share/fonts/truetype/noto/NotoSans-Regular.ttf",
        "/usr/share/fonts/TTF/DejaVuSans.ttf",
        "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
    ];
    const MEDIUM: &[&str] = &[
        "/usr/share/fonts/inter/Inter-Medium.ttf",
        "/usr/share/fonts/TTF/Inter-Medium.ttf",
        "/usr/share/fonts/noto/NotoSans-Medium.ttf",
        "/usr/share/fonts/truetype/noto/NotoSans-Medium.ttf",
    ];
    const BOLD: &[&str] = &[
        "/usr/share/fonts/inter/Inter-SemiBold.ttf",
        "/usr/share/fonts/TTF/Inter-SemiBold.ttf",
        "/usr/share/fonts/noto/NotoSans-Bold.ttf",
        "/usr/share/fonts/truetype/noto/NotoSans-Bold.ttf",
    ];

    let Some((regular_name, regular)) = load_first(REGULAR) else {
        return;
    };
    let mut fonts = FontDefinitions::default();
    fonts.font_data.insert(
        regular_name.to_owned(),
        std::sync::Arc::new(FontData::from_owned(regular)),
    );
    if let Some((name, bytes)) = load_first(MEDIUM) {
        fonts.font_data.insert(
            name.to_owned(),
            std::sync::Arc::new(FontData::from_owned(bytes)),
        );
        fonts.families.insert(
            family_medium(),
            vec![name.to_owned(), regular_name.to_owned()],
        );
    } else {
        fonts
            .families
            .insert(family_medium(), vec![regular_name.to_owned()]);
    }
    if let Some((name, bytes)) = load_first(BOLD) {
        fonts.font_data.insert(
            name.to_owned(),
            std::sync::Arc::new(FontData::from_owned(bytes)),
        );
        fonts.families.insert(
            FontFamily::Name("bold".into()),
            vec![name.to_owned(), regular_name.to_owned()],
        );
    }
    if let Some(family) = fonts.families.get_mut(&FontFamily::Proportional) {
        family.insert(0, regular_name.to_owned());
    }
    if let Some(family) = fonts.families.get_mut(&FontFamily::Monospace) {
        family.push(regular_name.to_owned());
    }
    ctx.set_fonts(fonts);
}

fn load_first(paths: &[&str]) -> Option<(&'static str, Vec<u8>)> {
    for path in paths {
        if let Ok(bytes) = std::fs::read(path) {
            if !bytes.is_empty() {
                let name = if path.contains("Inter") {
                    if path.contains("Medium") {
                        "Inter-Medium"
                    } else if path.contains("SemiBold") || path.contains("Bold") {
                        "Inter-Bold"
                    } else {
                        "Inter"
                    }
                } else if path.contains("NotoSans") {
                    if path.contains("Medium") {
                        "NotoSans-Medium"
                    } else if path.contains("Bold") {
                        "NotoSans-Bold"
                    } else {
                        "NotoSans"
                    }
                } else {
                    "DejaVuSans"
                };
                return Some((name, bytes));
            }
        }
    }
    None
}

pub fn accent_button(text: impl Into<String>) -> egui::Button<'static> {
    egui::Button::new(RichText::new(text).color(WHITE).size(SIZE_BODY))
        .fill(ACCENT)
        .stroke(Stroke::NONE)
        .corner_radius(RADIUS_CONTROL)
        .min_size(Vec2::new(80.0, 32.0))
}

pub fn standard_button(text: impl Into<String>) -> egui::Button<'static> {
    egui::Button::new(RichText::new(text).color(TEXT).size(SIZE_BODY))
        .fill(WHITE)
        .stroke(Stroke::new(1.0, BORDER))
        .corner_radius(RADIUS_CONTROL)
        .min_size(Vec2::new(80.0, 32.0))
}

pub fn danger_button(text: impl Into<String>) -> egui::Button<'static> {
    egui::Button::new(RichText::new(text).color(WHITE).size(SIZE_BODY))
        .fill(ERROR)
        .stroke(Stroke::NONE)
        .corner_radius(RADIUS_CONTROL)
        .min_size(Vec2::new(0.0, 32.0))
}

/// WinUI-style segmented toggle group. `selected` is true for the active item.
pub fn segmented_button(text: impl Into<String>, selected: bool) -> egui::Button<'static> {
    if selected {
        egui::Button::new(RichText::new(text).color(WHITE).size(SIZE_BODY))
            .fill(ACCENT)
            .stroke(Stroke::NONE)
            .corner_radius(RADIUS_CONTROL)
            .min_size(Vec2::new(0.0, 32.0))
    } else {
        egui::Button::new(RichText::new(text).color(TEXT).size(SIZE_BODY))
            .fill(WHITE)
            .stroke(Stroke::NONE)
            .corner_radius(RADIUS_CONTROL)
            .min_size(Vec2::new(0.0, 32.0))
    }
}

pub fn segmented_frame() -> Frame {
    Frame::new()
        .fill(WHITE)
        .stroke(Stroke::new(1.0, BORDER))
        .corner_radius(RADIUS_CONTROL)
        .inner_margin(Margin::same(2))
}

pub fn underline_focus(ui: &egui::Ui, response: &egui::Response) {
    let y = response.rect.bottom() - 0.5;
    let (width, color) = if response.has_focus() {
        (2.0, ACCENT)
    } else {
        (1.0, BORDER)
    };
    ui.painter().line_segment(
        [
            egui::pos2(response.rect.left() + 2.0, y),
            egui::pos2(response.rect.right() - 2.0, y),
        ],
        Stroke::new(width, color),
    );
}

pub fn dashed_rounded_rect(painter: &egui::Painter, rect: egui::Rect, radius: f32, stroke: Stroke) {
    let dash = 6.0;
    let gap = 4.0;
    let walk = |from: egui::Pos2, to: egui::Pos2| {
        let delta = to - from;
        let len = delta.length();
        if len <= 0.5 {
            return;
        }
        let dir = delta / len;
        let mut t = 0.0;
        while t < len {
            let end = (t + dash).min(len);
            painter.line_segment([from + dir * t, from + dir * end], stroke);
            t += dash + gap;
        }
    };
    let r = radius.min(rect.width() * 0.5).min(rect.height() * 0.5);
    let min = rect.min;
    let max = rect.max;
    walk(egui::pos2(min.x + r, min.y), egui::pos2(max.x - r, min.y));
    walk(egui::pos2(max.x, min.y + r), egui::pos2(max.x, max.y - r));
    walk(egui::pos2(max.x - r, max.y), egui::pos2(min.x + r, max.y));
    walk(egui::pos2(min.x, max.y - r), egui::pos2(min.x, min.y + r));
}

pub fn paint_image_glyph(painter: &egui::Painter, center: egui::Pos2, size: f32, color: Color32) {
    let rect = egui::Rect::from_center_size(center, Vec2::splat(size));
    painter.rect(
        rect,
        6.0,
        Color32::TRANSPARENT,
        Stroke::new(2.0, color),
        egui::StrokeKind::Middle,
    );
    painter.circle_filled(
        center + Vec2::new(-size * 0.18, -size * 0.16),
        size * 0.08,
        color,
    );
    let mountain = vec![
        center + Vec2::new(-size * 0.32, size * 0.28),
        center + Vec2::new(-size * 0.05, size * 0.02),
        center + Vec2::new(size * 0.12, size * 0.18),
        center + Vec2::new(size * 0.22, size * 0.08),
        center + Vec2::new(size * 0.32, size * 0.28),
    ];
    painter.line(mountain, Stroke::new(2.0, color));
}

pub fn pill(painter: &egui::Painter, pos: egui::Pos2, text: &str, right_align: bool) {
    let galley = painter.layout_no_wrap(
        text.to_string(),
        FontId::new(SIZE_CAPTION, FontFamily::Proportional),
        WHITE,
    );
    let size = galley.size() + Vec2::new(16.0, 8.0);
    let origin = if right_align {
        egui::pos2(pos.x - size.x, pos.y)
    } else {
        pos
    };
    let rect = egui::Rect::from_min_size(origin, size);
    painter.rect_filled(
        rect,
        999.0,
        Color32::from_rgba_unmultiplied(27, 27, 27, 170),
    );
    painter.galley(origin + Vec2::new(8.0, 4.0), galley, WHITE);
}

#[allow(dead_code)]
pub const SIZE_SECTION: f32 = 16.0;

#[allow(dead_code)]
pub fn section_text(text: impl Into<String>) -> RichText {
    RichText::new(text)
        .font(FontId::new(SIZE_SECTION, family_medium()))
        .color(TEXT)
}

#[allow(dead_code)]
pub fn slim_status_panel() -> Frame {
    Frame::new()
        .fill(MICA)
        .inner_margin(Margin::symmetric(16, 6))
        .stroke(Stroke::new(1.0, BORDER))
}

/// Clickable card header + optional body, Fluent-style.
#[allow(dead_code)]
pub fn collapsing_card<R>(
    ui: &mut egui::Ui,
    _id: impl std::hash::Hash,
    icon: crate::icons::Icon,
    title: &str,
    open: &mut bool,
    add_contents: impl FnOnce(&mut egui::Ui) -> R,
) -> Option<R> {
    use crate::icons;
    let mut result = None;
    card_frame().show(ui, |ui| {
        let header =
            ui.allocate_response(egui::vec2(ui.available_width(), 28.0), egui::Sense::click());
        let r = header.rect;
        icons::paint(
            ui,
            icon,
            egui::pos2(r.left() + 8.0, r.center().y),
            16.0,
            TEXT,
        );
        let galley = ui.painter().layout_no_wrap(
            title.to_string(),
            FontId::new(SIZE_SECTION, family_medium()),
            TEXT,
        );
        ui.painter().galley(
            egui::pos2(r.left() + 28.0, r.center().y - galley.size().y * 0.5),
            galley,
            TEXT,
        );
        icons::paint(
            ui,
            if *open {
                icons::Icon::ChevronDown
            } else {
                icons::Icon::ChevronRight
            },
            egui::pos2(r.right() - 12.0, r.center().y),
            14.0,
            TEXT_SECONDARY,
        );
        if header.clicked() {
            *open = !*open;
        }
        if *open {
            ui.add_space(4.0);
            result = Some(add_contents(ui));
        }
    });
    result
}

#[allow(dead_code)]
pub fn stepper(
    ui: &mut egui::Ui,
    value: &mut u32,
    range: std::ops::RangeInclusive<u32>,
) -> egui::Response {
    ui.add(
        egui::DragValue::new(value)
            .range(range)
            .speed(1)
            .update_while_editing(false),
    )
}

pub fn save_color_image(image: &egui::ColorImage, path: &std::path::Path) -> Result<(), String> {
    let [w, h] = image.size;
    let mut rgba = Vec::with_capacity(w * h * 4);
    // eframe/wgpu captures premultiplied pixels; composite onto Mica so
    // unpainted or translucent fragments never appear as a black strip.
    for pixel in &image.pixels {
        let [r, g, b, a] = pixel.to_array();
        let (r, g, b) = if a == 0 {
            (MICA.r(), MICA.g(), MICA.b())
        } else if a == 255 {
            (r, g, b)
        } else {
            let a16 = a as u16;
            let unpremul = |c: u8| ((c as u16 * 255 + a16 / 2) / a16).min(255) as u8;
            let (sr, sg, sb) = (unpremul(r), unpremul(g), unpremul(b));
            let inv = 255 - a16;
            (
                ((sr as u16 * a16 + MICA.r() as u16 * inv) / 255) as u8,
                ((sg as u16 * a16 + MICA.g() as u16 * inv) / 255) as u8,
                ((sb as u16 * a16 + MICA.b() as u16 * inv) / 255) as u8,
            )
        };
        rgba.extend_from_slice(&[r, g, b, 255]);
    }
    image::RgbaImage::from_raw(w as u32, h as u32, rgba)
        .ok_or_else(|| "invalid screenshot buffer".to_string())?
        .save(path)
        .map_err(|err| err.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fluent_palette_matches_the_spec() {
        assert_eq!(MICA, Color32::from_rgb(243, 243, 243));
        assert_eq!(CARD, Color32::from_rgb(251, 251, 251));
        assert_eq!(BORDER, Color32::from_rgb(229, 229, 229));
        assert_eq!(TEXT, Color32::from_rgb(27, 27, 27));
        assert_eq!(TEXT_SECONDARY, Color32::from_rgb(95, 95, 95));
        assert_eq!(ACCENT, Color32::from_rgb(0, 95, 184));
        assert_eq!(ERROR, Color32::from_rgb(196, 43, 28));
        assert_eq!(SUCCESS, Color32::from_rgb(15, 123, 15));
        assert_eq!(CAUTION, Color32::from_rgb(157, 93, 0));
        assert_eq!(RADIUS_CONTROL, 4);
        assert_eq!(RADIUS_CARD, 8);
    }
}
