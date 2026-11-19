use crate::extension::Field;
use crate::extensions::calculator::history::{civil_from_days, days_from_civil};
use crate::{icons, theme};
use egui::{pos2, vec2, Align2, Color32, Id, Key, Modifiers, Rect, Sense, Stroke, Ui};

const DAY_MS: i64 = 86_400_000;
const CELL: f32 = 32.0;
const PAD: f32 = 12.0;
const HEAD: f32 = 40.0;
const WEEKDAYS: f32 = 24.0;
const TIME_ROW: f32 = 48.0;
const MINUTE_STEP: i64 = 5;

#[derive(Clone, Copy, Default, PartialEq)]
enum Zone {
    #[default]
    Days,
    Hour,
    Minute,
}

#[derive(Clone, Copy, Default)]
struct State {
    open: bool,
    cursor: i64,
    zone: Zone,
}

pub fn is_open(ui: &Ui) -> bool {
    ui.memory(|m| m.focused()).and_then(|id| ui.data(|d| d.get_temp::<State>(id))).is_some_and(|s| s.open)
}

fn step(zone: Zone, forward: bool, time: bool) -> Zone {
    if !time {
        return Zone::Days;
    }
    match (zone, forward) {
        (Zone::Days, true) | (Zone::Minute, false) => Zone::Hour,
        (Zone::Hour, true) | (Zone::Days, false) => Zone::Minute,
        (Zone::Minute, true) | (Zone::Hour, false) => Zone::Days,
    }
}

fn label(ui: &Ui, rect: Rect, align: Align2, text: &str, font: egui::FontId, color: Color32) {
    let anchor = align.pos_in_rect(&rect);
    ui.painter().text(anchor, align, text, font, color);
}

fn icon_button(ui: &Ui, rect: Rect, id: Id, glyph: &str, name: &str) -> egui::Response {
    let response = ui.interact(rect, id, Sense::click());
    response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, name));
    if response.hovered() {
        ui.painter().rect_filled(rect, 4.0, theme::CONTROL_HOVER());
    }
    let size = (rect.height() - 8.0).clamp(10.0, 14.0);
    icons::paint_glyph(ui, glyph, Rect::from_center_size(rect.center(), vec2(size, size)), if response.hovered() { theme::FG() } else { theme::FG_60() });
    response
}

pub fn field(ui: &mut Ui, rect: Rect, field: &mut Field, id: Id, time: bool) -> bool {
    let pick_id = id.with("date-pick");
    let mut state: State = ui.data(|d| d.get_temp(pick_id)).unwrap_or_default();
    let (day, mut minutes) = parse(&field.value).or_else(|| parse(&field.placeholder)).unwrap_or((today(), 0));
    let selected = parse(&field.value).map(|(d, _)| d);
    let mut chosen: Option<i64> = None;
    let mut close = false;
    let text_focused = ui.memory(|m| m.has_focus(id));
    let open_now = |state: &mut State| {
        state.open = true;
        state.cursor = day;
        state.zone = Zone::Days;
    };
    if text_focused && ui.input_mut(|i| i.consume_key(Modifiers::ALT, Key::ArrowDown)) {
        open_now(&mut state);
    }

    let button_rect = Rect::from_min_size(pos2(rect.right() - 4.0 - 26.0, rect.center().y - 13.0), vec2(26.0, 26.0));
    let picker_focused = ui.memory(|m| m.has_focus(pick_id));
    let border = if field.error.is_some() { theme::RED() } else if text_focused || picker_focused { theme::FG_40() } else { theme::KEY_BORDER() };
    ui.painter().rect(rect, 8.0, theme::fg(3), Stroke::new(1.0, border), egui::StrokeKind::Inside);
    let inner = Rect::from_min_max(pos2(rect.left() + 12.0, rect.top() + 8.0), pos2(button_rect.left() - 6.0, rect.bottom() - 8.0));
    let hint = egui::RichText::new(field.placeholder.as_str()).color(theme::FG_40()).font(theme::font(13.0));
    let edit = egui::TextEdit::singleline(&mut field.value).id(id).frame(egui::Frame::NONE).margin(egui::Margin::ZERO).font(theme::font(13.0)).text_color(theme::FG()).hint_text(hint).desired_width(inner.width());
    let response = ui.put(inner, edit);
    response.widget_info(|| egui::WidgetInfo::text_edit(true, field.label, &field.value, ""));
    let mut changed = response.changed();
    if response.gained_focus() {
        response.scroll_to_me(None);
    }

    let name = if time { "Choose date and time" } else { "Choose date" };
    let button = icon_button(ui, button_rect, pick_id, "calendar", &format!("{name}: {}", field.label));
    if picker_focused {
        ui.painter().rect_stroke(button_rect.expand(1.0), 5.0, Stroke::new(1.0, theme::FG_40()), egui::StrokeKind::Outside);
    }
    let key_open = picker_focused && !state.open && ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Enter) || i.consume_key(Modifiers::NONE, Key::Space));
    if button.clicked() || key_open {
        if state.open && button.clicked() {
            close = true;
        } else {
            open_now(&mut state);
        }
    }

    let ctx = ui.ctx().clone();
    let size = vec2(7.0 * CELL + 2.0 * PAD, HEAD + WEEKDAYS + 6.0 * CELL + 8.0 + if time { TIME_ROW } else { 0.0 });
    let below = rect.bottom() + 4.0 + size.y <= ctx.content_rect().bottom();
    let frame = Rect::from_min_size(pos2(rect.left(), if below { rect.bottom() + 4.0 } else { rect.top() - 4.0 - size.y }), size);
    if state.open && ctx.input(|i| i.pointer.any_click() && i.pointer.interact_pos().is_some_and(|p| !frame.contains(p) && !button_rect.contains(p))) {
        state.open = false;
        ui.memory_mut(|m| m.set_focus_lock_filter(pick_id, egui::EventFilter::default()));
    }
    if state.open && !close {
        ui.memory_mut(|m| {
            m.request_focus(pick_id);
            m.set_focus_lock_filter(pick_id, egui::EventFilter { tab: true, horizontal_arrows: true, vertical_arrows: true, escape: true });
        });
        let pressed = |key: Key, mods: Modifiers| ui.input_mut(|i| i.consume_key(mods, key));
        if pressed(Key::Escape, Modifiers::NONE) {
            close = true;
        }
        if pressed(Key::Tab, Modifiers::NONE) {
            state.zone = step(state.zone, true, time);
        }
        if pressed(Key::Tab, Modifiers::SHIFT) {
            state.zone = step(state.zone, false, time);
        }
        let enter = pressed(Key::Enter, Modifiers::NONE);
        match state.zone {
            Zone::Days => {
                for (key, delta) in [(Key::ArrowLeft, -1), (Key::ArrowRight, 1), (Key::ArrowUp, -7), (Key::ArrowDown, 7)] {
                    if pressed(key, Modifiers::NONE) {
                        state.cursor += delta;
                    }
                }
                for (key, months) in [(Key::PageUp, -1), (Key::PageDown, 1)] {
                    if pressed(key, Modifiers::NONE) {
                        state.cursor = add_months(state.cursor, months);
                    }
                }
                if enter || pressed(Key::Space, Modifiers::NONE) {
                    chosen = Some(state.cursor);
                    close = true;
                }
            }
            zone => {
                let unit = if zone == Zone::Hour { 60 } else { MINUTE_STEP };
                for (key, sign) in [(Key::ArrowUp, 1), (Key::ArrowDown, -1)] {
                    if pressed(key, Modifiers::NONE) {
                        minutes = shift_minutes(minutes, sign * unit);
                        chosen = Some(selected.unwrap_or(day));
                    }
                }
                close |= enter;
            }
        }
    }

    if state.open && !close {
        let locale = crate::icu::display_locale();
        egui::Area::new(pick_id.with("popover")).order(egui::Order::Foreground).fixed_pos(frame.min).show(&ctx, |ui| {
            ui.painter().add(egui::epaint::RectShape::filled(frame.translate(vec2(0.0, 8.0)).expand(4.0), 10.0, Color32::from_black_alpha(40)));
            ui.painter().rect(frame, 6.0, theme::PANEL(), Stroke::new(1.0, theme::PANEL_BORDER()), egui::StrokeKind::Inside);
            let head = Rect::from_min_size(frame.min, vec2(frame.width(), HEAD));
            let title = crate::icu::format_skeleton(noon_ms(state.cursor), &locale, "yMMMM", Some("UTC"));
            label(ui, head.shrink2(vec2(PAD + 4.0, 0.0)), Align2::LEFT_CENTER, &title, theme::weighted(13.0, theme::MEDIUM), theme::FG());
            let next = Rect::from_min_size(pos2(head.right() - PAD - 24.0, head.center().y - 12.0), vec2(24.0, 24.0));
            let prev = next.translate(vec2(-28.0, 0.0));
            if icon_button(ui, prev, pick_id.with("prev"), "chevron-left", "Previous month").clicked() {
                state.cursor = add_months(state.cursor, -1);
            }
            if icon_button(ui, next, pick_id.with("next"), "chevron-right", "Next month").clicked() {
                state.cursor = add_months(state.cursor, 1);
            }
            let first = grid_start(state.cursor);
            let month = civil_from_days(state.cursor).1;
            let cell = |i: i64| Rect::from_min_size(pos2(frame.left() + PAD + (i % 7) as f32 * CELL, frame.top() + HEAD + WEEKDAYS + (i / 7) as f32 * CELL), vec2(CELL, CELL));
            for i in 0..7 {
                let r = cell(i).translate(vec2(0.0, -WEEKDAYS));
                let name = crate::icu::format_skeleton(noon_ms(first + i), &locale, "EEEEEE", Some("UTC"));
                label(ui, Rect::from_min_size(r.min, vec2(CELL, WEEKDAYS)), Align2::CENTER_CENTER, &name, theme::font(11.0), theme::FG_40());
            }
            let now = today();
            for i in 0..42 {
                let d = first + i;
                let r = cell(i).shrink(2.0);
                let response = ui.interact(r, pick_id.with(("day", d)), Sense::click());
                let iso = format(d, 0, false);
                response.widget_info(|| egui::WidgetInfo::selected(egui::Role::Button, true, selected == Some(d), &iso));
                let is_selected = selected == Some(d);
                if is_selected {
                    ui.painter().rect_filled(r, 6.0, theme::ACCENT());
                } else if (d == state.cursor && state.zone == Zone::Days) || response.hovered() {
                    ui.painter().rect_filled(r, 6.0, theme::PANEL_LINE());
                }
                if d == state.cursor && state.zone == Zone::Days {
                    ui.painter().rect_stroke(r, 6.0, Stroke::new(1.0, theme::FG_40()), egui::StrokeKind::Inside);
                }
                let color = if is_selected { Color32::WHITE } else if d == now { theme::ACCENT() } else if civil_from_days(d).1 != month { theme::FG_40() } else { theme::FG() };
                let font = if d == now { theme::weighted(13.0, theme::SEMIBOLD) } else { theme::font(13.0) };
                label(ui, r, Align2::CENTER_CENTER, &civil_from_days(d).2.to_string(), font, color);
                if response.clicked() {
                    state.cursor = d;
                    chosen = Some(d);
                    close = !time;
                }
            }
            if time {
                let row = Rect::from_min_max(pos2(frame.left(), frame.bottom() - TIME_ROW), frame.max);
                ui.painter().hline(frame.x_range().shrink(1.0), row.top() + 0.5, Stroke::new(1.0, theme::PANEL_LINE()));
                icons::paint_glyph(ui, "clock", Rect::from_min_size(pos2(row.left() + PAD + 4.0, row.center().y - 8.0), vec2(16.0, 16.0)), theme::FG_60());
                let mut x = row.left() + PAD + 4.0 + 16.0 + 10.0;
                for (zone, value, unit, name) in [(Zone::Hour, minutes / 60, 60, "Hour"), (Zone::Minute, minutes % 60, MINUTE_STEP, "Minute")] {
                    let seg = Rect::from_min_size(pos2(x, row.center().y - 14.0), vec2(56.0, 28.0));
                    let response = ui.interact(seg, pick_id.with(name), Sense::click());
                    response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::SpinButton, true, format!("{name}: {value:02}")));
                    let active = state.zone == zone;
                    ui.painter().rect(seg, 6.0, if active || response.hovered() { theme::PANEL_LINE() } else { Color32::TRANSPARENT }, Stroke::new(1.0, if active { theme::FG_40() } else { theme::KEY_BORDER() }), egui::StrokeKind::Inside);
                    label(ui, Rect::from_min_max(seg.min, pos2(seg.right() - 18.0, seg.bottom())), Align2::CENTER_CENTER, &format!("{value:02}"), theme::weighted(13.0, theme::MEDIUM), theme::FG());
                    let up = Rect::from_min_size(pos2(seg.right() - 18.0, seg.top() + 1.0), vec2(16.0, 13.0));
                    let down = up.translate(vec2(0.0, 13.0));
                    if icon_button(ui, up, pick_id.with((name, "up")), "chevron-up", &format!("Increase {name}")).clicked() {
                        minutes = shift_minutes(minutes, unit);
                        chosen = Some(selected.unwrap_or(day));
                    }
                    if icon_button(ui, down, pick_id.with((name, "down")), "chevron-down", &format!("Decrease {name}")).clicked() {
                        minutes = shift_minutes(minutes, -unit);
                        chosen = Some(selected.unwrap_or(day));
                    }
                    if response.clicked() {
                        state.zone = zone;
                    }
                    if zone == Zone::Hour {
                        label(ui, Rect::from_min_size(pos2(seg.right(), seg.top()), vec2(12.0, seg.height())), Align2::CENTER_CENTER, ":", theme::weighted(13.0, theme::MEDIUM), theme::FG_60());
                    }
                    x = seg.right() + 12.0;
                }
            }
        });
    }

    if let Some(d) = chosen {
        field.value = format(d, minutes, time);
        field.error = None;
        changed = true;
    }
    if close {
        state.open = false;
        ui.memory_mut(|m| {
            m.request_focus(pick_id);
            m.set_focus_lock_filter(pick_id, egui::EventFilter::default());
        });
    }
    ui.data_mut(|d| d.insert_temp(pick_id, state));
    changed
}

pub fn parse(text: &str) -> Option<(i64, i64)> {
    let text = text.trim();
    let (date, time) = text.split_once([' ', 'T']).map_or((text, None), |(d, t)| (d, Some(t.trim())));
    let mut parts = date.split('-').map(|p| p.parse::<i64>().ok());
    let (y, m, d) = (parts.next()??, parts.next()??, parts.next()??);
    if parts.next().is_some() || !(1..=12).contains(&m) || !(1..=days_in_month(y, m)).contains(&d) {
        return None;
    }
    let minutes = match time {
        Some(t) => {
            let (h, min) = t.split_once(':')?;
            let h = h.parse::<i64>().ok().filter(|h| (0..24).contains(h))?;
            let min = min.parse::<i64>().ok().filter(|m| (0..60).contains(m))?;
            h * 60 + min
        }
        None => 0,
    };
    Some((days_from_civil(y, m, d), minutes))
}

pub fn format(day: i64, minutes: i64, time: bool) -> String {
    let (y, m, d) = civil_from_days(day);
    if time {
        format!("{y:04}-{m:02}-{d:02} {:02}:{:02}", minutes / 60, minutes % 60)
    } else {
        format!("{y:04}-{m:02}-{d:02}")
    }
}

pub fn days_in_month(year: i64, month: i64) -> i64 {
    days_from_civil(year, month + 1, 1) - days_from_civil(year, month, 1)
}

pub fn add_months(day: i64, months: i64) -> i64 {
    let (y, m, d) = civil_from_days(day);
    let total = y * 12 + m - 1 + months;
    let (y, m) = (total.div_euclid(12), total.rem_euclid(12) + 1);
    days_from_civil(y, m, d.min(days_in_month(y, m)))
}

pub fn month_start(day: i64) -> i64 {
    let (y, m, _) = civil_from_days(day);
    days_from_civil(y, m, 1)
}

pub fn grid_start(day: i64) -> i64 {
    let first = month_start(day);
    first - (first + 3).rem_euclid(7)
}

pub fn shift_minutes(minutes: i64, delta: i64) -> i64 {
    (minutes + delta).rem_euclid(24 * 60)
}

pub fn today() -> i64 {
    let now = crate::store::now_ms() as i64;
    (now + crate::icu::local_offset_minutes(now as f64) as i64 * 60_000).div_euclid(DAY_MS)
}

pub fn noon_ms(day: i64) -> f64 {
    (day * DAY_MS + DAY_MS / 2) as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_dates_and_times_strictly() {
        let day = days_from_civil(2026, 10, 21);
        assert_eq!(parse("2026-10-21 09:05"), Some((day, 9 * 60 + 5)));
        assert_eq!(parse(" 2026-10-21T23:59 "), Some((day, 23 * 60 + 59)));
        assert_eq!(parse("2026-10-21"), Some((day, 0)));
        assert_eq!(parse("2024-02-29"), Some((days_from_civil(2024, 2, 29), 0)));
        for bad in ["", "2026-13-01", "2026-02-29", "2026-04-31", "2026-10-21 24:00", "2026-10-21 9", "2026-10-21-1", "tomorrow"] {
            assert_eq!(parse(bad), None, "{bad}");
        }
    }

    #[test]
    fn formats_round_trip() {
        let day = days_from_civil(2026, 1, 5);
        assert_eq!(format(day, 7 * 60 + 3, true), "2026-01-05 07:03");
        assert_eq!(format(day, 7 * 60 + 3, false), "2026-01-05");
        assert_eq!(parse(&format(day, 615, true)), Some((day, 615)));
    }

    #[test]
    fn steps_months_clamping_the_day() {
        assert_eq!(add_months(days_from_civil(2026, 1, 31), 1), days_from_civil(2026, 2, 28));
        assert_eq!(add_months(days_from_civil(2024, 1, 31), 1), days_from_civil(2024, 2, 29));
        assert_eq!(add_months(days_from_civil(2026, 12, 15), 1), days_from_civil(2027, 1, 15));
        assert_eq!(add_months(days_from_civil(2026, 1, 15), -1), days_from_civil(2025, 12, 15));
        assert_eq!(add_months(days_from_civil(2026, 3, 31), -13), days_from_civil(2025, 2, 28));
        assert_eq!(days_in_month(2026, 12), 31);
        assert_eq!(days_in_month(2100, 2), 28);
    }

    #[test]
    fn grid_starts_on_the_monday_on_or_before_the_first() {
        assert_eq!(grid_start(days_from_civil(2026, 11, 19)), days_from_civil(2026, 10, 26));
        assert_eq!(grid_start(days_from_civil(2026, 6, 30)), days_from_civil(2026, 6, 1));
        assert_eq!(month_start(days_from_civil(2026, 11, 19)), days_from_civil(2026, 11, 1));
    }

    #[test]
    fn tab_cycles_days_hour_minute_only_with_time() {
        assert!(step(Zone::Days, true, true) == Zone::Hour && step(Zone::Hour, true, true) == Zone::Minute && step(Zone::Minute, true, true) == Zone::Days);
        assert!(step(Zone::Days, false, true) == Zone::Minute && step(Zone::Hour, false, true) == Zone::Days);
        assert!(step(Zone::Days, true, false) == Zone::Days);
    }

    #[test]
    fn wraps_time_steps_within_a_day() {
        assert_eq!(shift_minutes(23 * 60 + 30, 60), 30);
        assert_eq!(shift_minutes(2, -5), 24 * 60 - 3);
        assert_eq!(shift_minutes(9 * 60, 5), 9 * 60 + 5);
    }
}
