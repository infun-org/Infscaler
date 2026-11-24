//! Small Fluent-style vector icons drawn with egui's painter.
//! Avoids icon fonts (and tofu) so the same shapes work with Noto Sans.

#![allow(dead_code)] // used by the upcoming layout rewrite
use eframe::egui::{self, Color32, CornerRadius, Pos2, Rect, Sense, Stroke, StrokeKind, Ui, Vec2};

use crate::theme;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Icon {
    Image,
    Sparkle,
    Resize,
    Gpu,
    Folder,
    Open,
    Drop,
    Globe,
    Refresh,
    More,
    Play,
    Fit,
    Minus,
    Plus,
    Hand,
    Compare,
    Check,
    Queue,
    ChevronDown,
    ChevronUp,
    ChevronRight,
    Warning,
    Close,
}

pub fn paint(ui: &Ui, icon: Icon, center: Pos2, size: f32, color: Color32) {
    let painter = ui.painter();
    let s = size * 0.5;
    let stroke = Stroke::new((size * 0.1).clamp(1.25, 1.8), color);
    match icon {
        Icon::Image => {
            let r = Rect::from_center_size(center, Vec2::splat(size * 0.78));
            painter.rect(r, 2.5, Color32::TRANSPARENT, stroke, StrokeKind::Middle);
            painter.circle_filled(center + Vec2::new(-s * 0.28, -s * 0.22), size * 0.07, color);
            let mountain = vec![
                center + Vec2::new(-s * 0.62, s * 0.52),
                center + Vec2::new(-s * 0.18, s * 0.02),
                center + Vec2::new(s * 0.08, s * 0.28),
                center + Vec2::new(s * 0.28, s * 0.1),
                center + Vec2::new(s * 0.62, s * 0.52),
            ];
            painter.line(mountain, stroke);
        }
        Icon::Sparkle => {
            // Four-point sparkle + a smaller one.
            let star = |c: Pos2, arm: f32| {
                let pts = [
                    c + Vec2::new(0.0, -arm),
                    c + Vec2::new(arm * 0.22, -arm * 0.22),
                    c + Vec2::new(arm, 0.0),
                    c + Vec2::new(arm * 0.22, arm * 0.22),
                    c + Vec2::new(0.0, arm),
                    c + Vec2::new(-arm * 0.22, arm * 0.22),
                    c + Vec2::new(-arm, 0.0),
                    c + Vec2::new(-arm * 0.22, -arm * 0.22),
                    c + Vec2::new(0.0, -arm),
                ];
                painter.line(pts.to_vec(), stroke);
            };
            star(center + Vec2::new(-s * 0.12, -s * 0.08), s * 0.62);
            star(center + Vec2::new(s * 0.5, s * 0.42), s * 0.28);
        }
        Icon::Resize => {
            // Diagonal arrows from corner to corner.
            let r = size * 0.38;
            let a = center + Vec2::new(-r, r);
            let b = center + Vec2::new(r, -r);
            painter.line_segment([a, b], stroke);
            // arrow heads
            painter.line_segment([a, a + Vec2::new(0.0, -s * 0.32)], stroke);
            painter.line_segment([a, a + Vec2::new(s * 0.32, 0.0)], stroke);
            painter.line_segment([b, b + Vec2::new(0.0, s * 0.32)], stroke);
            painter.line_segment([b, b + Vec2::new(-s * 0.32, 0.0)], stroke);
        }
        Icon::Gpu => {
            let r = Rect::from_center_size(center, Vec2::new(size * 0.72, size * 0.5));
            painter.rect(r, 2.0, Color32::TRANSPARENT, stroke, StrokeKind::Middle);
            // pins
            for i in 0..4 {
                let t = (i as f32 + 0.5) / 4.0;
                let x = r.left() + r.width() * t;
                painter.line_segment(
                    [Pos2::new(x, r.top()), Pos2::new(x, r.top() - s * 0.28)],
                    stroke,
                );
                painter.line_segment(
                    [
                        Pos2::new(x, r.bottom()),
                        Pos2::new(x, r.bottom() + s * 0.28),
                    ],
                    stroke,
                );
            }
        }
        Icon::Folder => {
            let top = Rect::from_min_size(
                center + Vec2::new(-s * 0.7, -s * 0.45),
                Vec2::new(s * 0.55, s * 0.28),
            );
            painter.rect(top, 1.5, Color32::TRANSPARENT, stroke, StrokeKind::Middle);
            let body = Rect::from_min_size(
                center + Vec2::new(-s * 0.7, -s * 0.28),
                Vec2::new(s * 1.4, s * 0.95),
            );
            painter.rect(body, 2.0, Color32::TRANSPARENT, stroke, StrokeKind::Middle);
        }
        Icon::Open => {
            // Landscape image with a corner fold — "open photo".
            let r = Rect::from_center_size(center, Vec2::new(size * 0.78, size * 0.62));
            painter.rect(r, 2.0, Color32::TRANSPARENT, stroke, StrokeKind::Middle);
            painter.circle_filled(center + Vec2::new(-s * 0.22, -s * 0.12), size * 0.06, color);
            painter.line(
                vec![
                    Pos2::new(r.left(), r.bottom()),
                    center + Vec2::new(-s * 0.05, s * 0.05),
                    center + Vec2::new(s * 0.2, s * 0.28),
                    Pos2::new(r.right(), r.bottom() - s * 0.15),
                ],
                stroke,
            );
        }
        Icon::Drop => {
            // Tray with a downward arrow.
            let tip = center + Vec2::new(0.0, s * 0.18);
            painter.line_segment([center + Vec2::new(0.0, -s * 0.55), tip], stroke);
            painter.line_segment([tip, tip + Vec2::new(-s * 0.28, -s * 0.28)], stroke);
            painter.line_segment([tip, tip + Vec2::new(s * 0.28, -s * 0.28)], stroke);
            let y = center.y + s * 0.55;
            painter.line(
                vec![
                    Pos2::new(center.x - s * 0.62, y - s * 0.22),
                    Pos2::new(center.x - s * 0.62, y),
                    Pos2::new(center.x + s * 0.62, y),
                    Pos2::new(center.x + s * 0.62, y - s * 0.22),
                ],
                stroke,
            );
        }
        Icon::Globe => {
            painter.circle_stroke(center, s * 0.68, stroke);
            painter.circle_stroke(
                center,
                s * 0.68,
                stroke, // meridians as ellipses via clipped arcs approx
            );
            // equator
            painter.line_segment(
                [
                    center + Vec2::new(-s * 0.68, 0.0),
                    center + Vec2::new(s * 0.68, 0.0),
                ],
                stroke,
            );
            // latitude hints
            painter.line_segment(
                [
                    center + Vec2::new(-s * 0.55, -s * 0.32),
                    center + Vec2::new(s * 0.55, -s * 0.32),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    center + Vec2::new(-s * 0.55, s * 0.32),
                    center + Vec2::new(s * 0.55, s * 0.32),
                ],
                stroke,
            );
            // meridian (vertical ellipse approximated by vertical line + two curves)
            painter.line_segment(
                [
                    center + Vec2::new(0.0, -s * 0.68),
                    center + Vec2::new(0.0, s * 0.68),
                ],
                stroke,
            );
        }
        Icon::Refresh => {
            let radius = s * 0.55;
            // 3/4 arc
            let mut pts = Vec::new();
            let start = std::f32::consts::PI * 0.15;
            let end = std::f32::consts::PI * 1.85;
            let steps = 16;
            for i in 0..=steps {
                let t = start + (end - start) * (i as f32 / steps as f32);
                pts.push(center + Vec2::new(t.cos(), t.sin()) * radius);
            }
            painter.line(pts, stroke);
            let tip = center + Vec2::new(start.cos(), start.sin()) * radius;
            painter.line_segment([tip, tip + Vec2::new(-s * 0.28, -s * 0.06)], stroke);
            painter.line_segment([tip, tip + Vec2::new(s * 0.04, -s * 0.3)], stroke);
        }
        Icon::More => {
            for dy in [-0.42, 0.0, 0.42] {
                painter.circle_filled(center + Vec2::new(0.0, s * dy), size * 0.07, color);
            }
        }
        Icon::Play => {
            let pts = vec![
                center + Vec2::new(-s * 0.28, -s * 0.42),
                center + Vec2::new(s * 0.48, 0.0),
                center + Vec2::new(-s * 0.28, s * 0.42),
                center + Vec2::new(-s * 0.28, -s * 0.42),
            ];
            painter.line(pts, stroke);
        }
        Icon::Fit => {
            let arm = s * 0.32;
            let inset = s * 0.62;
            let corners = [
                (
                    Vec2::new(-inset, -inset),
                    Vec2::new(arm, 0.0),
                    Vec2::new(0.0, arm),
                ),
                (
                    Vec2::new(inset, -inset),
                    Vec2::new(-arm, 0.0),
                    Vec2::new(0.0, arm),
                ),
                (
                    Vec2::new(-inset, inset),
                    Vec2::new(arm, 0.0),
                    Vec2::new(0.0, -arm),
                ),
                (
                    Vec2::new(inset, inset),
                    Vec2::new(-arm, 0.0),
                    Vec2::new(0.0, -arm),
                ),
            ];
            for (c, a, b) in corners {
                let p = center + c;
                painter.line_segment([p, p + a], stroke);
                painter.line_segment([p, p + b], stroke);
            }
        }
        Icon::Minus => {
            painter.line_segment(
                [
                    center + Vec2::new(-s * 0.42, 0.0),
                    center + Vec2::new(s * 0.42, 0.0),
                ],
                stroke,
            );
        }
        Icon::Plus => {
            painter.line_segment(
                [
                    center + Vec2::new(-s * 0.42, 0.0),
                    center + Vec2::new(s * 0.42, 0.0),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    center + Vec2::new(0.0, -s * 0.42),
                    center + Vec2::new(0.0, s * 0.42),
                ],
                stroke,
            );
        }
        Icon::Hand => {
            // Simple open-hand outline.
            let palm = Rect::from_center_size(
                center + Vec2::new(0.0, s * 0.18),
                Vec2::new(s * 0.7, s * 0.7),
            );
            painter.rect(palm, 3.0, Color32::TRANSPARENT, stroke, StrokeKind::Middle);
            // fingers
            for i in 0..3 {
                let x = palm.left() + palm.width() * ((i as f32 + 0.7) / 4.0);
                painter.line_segment(
                    [
                        Pos2::new(x, palm.top()),
                        Pos2::new(x, palm.top() - s * 0.45),
                    ],
                    stroke,
                );
            }
        }
        Icon::Compare => {
            let r = Rect::from_center_size(center, Vec2::new(size * 0.82, size * 0.64));
            painter.rect(r, 2.0, Color32::TRANSPARENT, stroke, StrokeKind::Middle);
            let mid = r.center().x;
            painter.line_segment(
                [Pos2::new(mid, r.top()), Pos2::new(mid, r.bottom())],
                stroke,
            );
        }
        Icon::Check => {
            painter.line(
                vec![
                    center + Vec2::new(-s * 0.4, 0.05),
                    center + Vec2::new(-s * 0.08, s * 0.38),
                    center + Vec2::new(s * 0.48, -s * 0.38),
                ],
                stroke,
            );
        }
        Icon::Queue => {
            // Stacked layers.
            for i in 0..3 {
                let o = (i as f32 - 1.0) * s * 0.22;
                let r = Rect::from_center_size(
                    center + Vec2::new(0.0, o),
                    Vec2::new(size * 0.78, size * 0.28),
                );
                painter.rect(r, 1.5, Color32::TRANSPARENT, stroke, StrokeKind::Middle);
            }
        }
        Icon::ChevronDown => {
            painter.line(
                vec![
                    center + Vec2::new(-s * 0.4, -s * 0.15),
                    center + Vec2::new(0.0, s * 0.28),
                    center + Vec2::new(s * 0.4, -s * 0.15),
                ],
                stroke,
            );
        }
        Icon::ChevronUp => {
            painter.line(
                vec![
                    center + Vec2::new(-s * 0.4, s * 0.15),
                    center + Vec2::new(0.0, -s * 0.28),
                    center + Vec2::new(s * 0.4, s * 0.15),
                ],
                stroke,
            );
        }
        Icon::ChevronRight => {
            painter.line(
                vec![
                    center + Vec2::new(-s * 0.15, -s * 0.4),
                    center + Vec2::new(s * 0.28, 0.0),
                    center + Vec2::new(-s * 0.15, s * 0.4),
                ],
                stroke,
            );
        }
        Icon::Warning => {
            painter.line(
                vec![
                    center + Vec2::new(0.0, -s * 0.7),
                    center + Vec2::new(s * 0.68, s * 0.55),
                    center + Vec2::new(-s * 0.68, s * 0.55),
                    center + Vec2::new(0.0, -s * 0.7),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    center + Vec2::new(0.0, -s * 0.22),
                    center + Vec2::new(0.0, s * 0.18),
                ],
                stroke,
            );
            painter.circle_filled(center + Vec2::new(0.0, s * 0.38), size * 0.06, color);
        }
        Icon::Close => {
            painter.line_segment(
                [
                    center + Vec2::new(-s * 0.4, -s * 0.4),
                    center + Vec2::new(s * 0.4, s * 0.4),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    center + Vec2::new(s * 0.4, -s * 0.4),
                    center + Vec2::new(-s * 0.4, s * 0.4),
                ],
                stroke,
            );
        }
    }
}

/// Square toolbar / header icon button.
pub fn icon_button(ui: &mut Ui, icon: Icon, selected: bool, tip: &str) -> egui::Response {
    let size = 32.0;
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(size), Sense::click());
    let hovered = response.hovered();
    let (fill, stroke, color) = if selected {
        (theme::ACCENT, Stroke::NONE, theme::WHITE)
    } else if hovered {
        (
            theme::HOVER,
            Stroke::new(1.0, theme::BORDER_DARK),
            theme::TEXT,
        )
    } else {
        (theme::WHITE, Stroke::new(1.0, theme::BORDER), theme::TEXT)
    };
    ui.painter().rect(
        rect,
        CornerRadius::same(theme::RADIUS_CONTROL),
        fill,
        stroke,
        StrokeKind::Inside,
    );
    paint(ui, icon, rect.center(), 16.0, color);
    response.on_hover_text(tip)
}

/// Accent or standard button with a leading icon and label.
pub fn labeled(
    ui: &mut Ui,
    icon: Icon,
    text: &str,
    accent: bool,
    enabled: bool,
    min_w: f32,
) -> egui::Response {
    let galley = ui.painter().layout_no_wrap(
        text.to_string(),
        egui::FontId::proportional(theme::SIZE_BODY),
        if accent { theme::WHITE } else { theme::TEXT },
    );
    let pad_x = 12.0;
    let icon_s = 16.0;
    let w = (pad_x + icon_s + 8.0 + galley.size().x + pad_x).max(min_w);
    let h = 32.0;
    let sense = if enabled {
        Sense::click()
    } else {
        Sense::hover()
    };
    let (rect, response) = ui.allocate_exact_size(Vec2::new(w, h), sense);
    let hovered = enabled && response.hovered();
    let (fill, stroke, fg) = if !enabled {
        (
            if accent {
                theme::ACCENT.gamma_multiply(0.45)
            } else {
                theme::WHITE
            },
            if accent {
                Stroke::NONE
            } else {
                Stroke::new(1.0, theme::BORDER)
            },
            if accent {
                theme::WHITE.gamma_multiply(0.85)
            } else {
                theme::TEXT_DISABLED
            },
        )
    } else if accent {
        let fill = if response.is_pointer_button_down_on() {
            theme::ACCENT_PRESSED
        } else if hovered {
            theme::ACCENT_HOVER
        } else {
            theme::ACCENT
        };
        (fill, Stroke::NONE, theme::WHITE)
    } else if hovered {
        (
            theme::HOVER,
            Stroke::new(1.0, theme::BORDER_DARK),
            theme::TEXT,
        )
    } else {
        (theme::WHITE, Stroke::new(1.0, theme::BORDER), theme::TEXT)
    };
    ui.painter().rect(
        rect,
        CornerRadius::same(theme::RADIUS_CONTROL),
        fill,
        stroke,
        StrokeKind::Inside,
    );
    let icon_c = Pos2::new(rect.left() + pad_x + icon_s * 0.5, rect.center().y);
    paint(ui, icon, icon_c, icon_s, fg);
    let text_pos = Pos2::new(
        rect.left() + pad_x + icon_s + 8.0,
        rect.center().y - galley.size().y * 0.5,
    );
    ui.painter().galley(text_pos, galley, fg);
    response
}

pub fn dashed_label_button(ui: &mut Ui, icon: Icon, text: &str, highlight: bool) -> egui::Response {
    let galley = ui.painter().layout_no_wrap(
        text.to_string(),
        egui::FontId::proportional(theme::SIZE_BODY),
        if highlight {
            theme::ACCENT
        } else {
            theme::TEXT_SECONDARY
        },
    );
    let pad_x = 12.0;
    let icon_s = 16.0;
    let w = (ui.available_width()).max(pad_x + icon_s + 8.0 + galley.size().x + pad_x);
    let h = 32.0;
    let (rect, response) = ui.allocate_exact_size(Vec2::new(w, h), Sense::click());
    let hovered = response.hovered() || highlight;
    let color = if highlight || hovered {
        theme::ACCENT
    } else {
        theme::TEXT_SECONDARY
    };
    let fill = if hovered {
        theme::ACCENT_TINT
    } else {
        theme::WHITE
    };
    ui.painter()
        .rect_filled(rect, CornerRadius::same(theme::RADIUS_CONTROL), fill);
    theme::dashed_rounded_rect(
        ui.painter(),
        rect.shrink(0.5),
        theme::RADIUS_CONTROL as f32,
        Stroke::new(
            1.25,
            if highlight || hovered {
                theme::ACCENT
            } else {
                theme::BORDER_DARK
            },
        ),
    );
    let icon_c = Pos2::new(rect.left() + pad_x + icon_s * 0.5, rect.center().y);
    paint(ui, icon, icon_c, icon_s, color);
    let text_pos = Pos2::new(
        rect.left() + pad_x + icon_s + 8.0,
        rect.center().y - galley.size().y * 0.5,
    );
    ui.painter().galley(text_pos, galley, color);
    response
}

pub fn paint_spinner(ui: &Ui, center: Pos2, radius: f32, color: Color32) {
    let t = ui.input(|i| i.time) as f32;
    let start = t * 4.0;
    let mut pts = Vec::new();
    let sweep = std::f32::consts::PI * 1.4;
    let steps = 18;
    for i in 0..=steps {
        let a = start + sweep * (i as f32 / steps as f32);
        pts.push(center + Vec2::new(a.cos(), a.sin()) * radius);
    }
    ui.painter().line(pts, Stroke::new(2.0, color));
}
