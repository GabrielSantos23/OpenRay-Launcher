use super::doc::{Block, Doc, Kind, Removal, Style, BOLD, CODE, ITALIC, STRIKE, UNDERLINE};
use crate::theme;
use egui::text::{CCursor, CCursorRange, LayoutJob, TextFormat};
use egui::text_edit::TextEditState;
use egui::{pos2, vec2, Color32, Galley, Key, KeyboardShortcut, Modifiers, Rect, Shape, Stroke, TextBuffer, Ui};
use std::sync::Arc;

const BODY: f32 = 14.0;
const BLOCK_GAP: f32 = 8.0;
const CODE_PAD: f32 = 8.0;
const UNDO_LIMIT: usize = 200;

#[derive(Clone)]
struct Snapshot {
    doc: Doc,
    cursor: usize,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Format {
    Mark(u8),
    Block(Kind),
}

pub const FORMAT_SHORTCUTS: [(Format, KeyboardShortcut); 13] = [
    (Format::Mark(BOLD), KeyboardShortcut::new(Modifiers::COMMAND, Key::B)),
    (Format::Mark(ITALIC), KeyboardShortcut::new(Modifiers::COMMAND, Key::I)),
    (Format::Mark(UNDERLINE), KeyboardShortcut::new(Modifiers::COMMAND, Key::U)),
    (Format::Mark(STRIKE), KeyboardShortcut::new(Modifiers::COMMAND.plus(Modifiers::SHIFT), Key::S)),
    (Format::Mark(CODE), KeyboardShortcut::new(Modifiers::COMMAND, Key::E)),
    (Format::Block(Kind::Paragraph), KeyboardShortcut::new(Modifiers::COMMAND.plus(Modifiers::ALT), Key::Num0)),
    (Format::Block(Kind::Heading(1)), KeyboardShortcut::new(Modifiers::COMMAND.plus(Modifiers::ALT), Key::Num1)),
    (Format::Block(Kind::Heading(2)), KeyboardShortcut::new(Modifiers::COMMAND.plus(Modifiers::ALT), Key::Num2)),
    (Format::Block(Kind::Heading(3)), KeyboardShortcut::new(Modifiers::COMMAND.plus(Modifiers::ALT), Key::Num3)),
    (Format::Block(Kind::Bullet), KeyboardShortcut::new(Modifiers::COMMAND.plus(Modifiers::SHIFT), Key::Num8)),
    (Format::Block(Kind::Ordered), KeyboardShortcut::new(Modifiers::COMMAND.plus(Modifiers::SHIFT), Key::Num7)),
    (Format::Block(Kind::Task(false)), KeyboardShortcut::new(Modifiers::COMMAND.plus(Modifiers::SHIFT), Key::Num9)),
    (Format::Block(Kind::Quote), KeyboardShortcut::new(Modifiers::COMMAND.plus(Modifiers::SHIFT), Key::B)),
];

pub struct NoteEditor {
    pub doc: Doc,
    undo: Vec<Snapshot>,
    redo: Vec<Snapshot>,
    pending: Option<Style>,
    pending_at: usize,
    last_push: f64,
    id: egui::Id,
}

struct LineGeometry {
    rows: Rect,
    first_row: Rect,
}

pub fn indent_x(block: Block) -> f32 {
    let level = block.indent as f32 * 22.0;
    match block.kind {
        Kind::Bullet | Kind::Ordered => level + 22.0,
        Kind::Task(_) => level + 24.0,
        Kind::Quote => 14.0,
        Kind::Code => 12.0,
        _ => 0.0,
    }
}

fn gap_before(prev: Option<Block>, block: Block) -> f32 {
    let Some(prev) = prev else {
        return if block.kind == Kind::Code { CODE_PAD } else { 0.0 };
    };
    let same_group = block.joined || (prev.kind.is_list() && block.kind.is_list()) || (prev.kind == block.kind && matches!(block.kind, Kind::Code | Kind::Quote));
    if same_group {
        return 0.0;
    }
    let margin = match block.kind {
        Kind::Heading(2) => 16.0,
        Kind::Heading(3) => 12.0,
        Kind::Rule => 16.0,
        _ => BLOCK_GAP,
    };
    let margin = if prev.kind == Kind::Rule { margin.max(16.0) } else { margin };
    margin + if prev.kind == Kind::Code { CODE_PAD } else { 0.0 } + if block.kind == Kind::Code { CODE_PAD } else { 0.0 }
}

fn base_format(block: Block) -> (egui::FontId, f32, Color32) {
    match block.kind {
        Kind::Heading(1) => (theme::weighted(24.0, theme::BOLD), 30.0, theme::FG()),
        Kind::Heading(2) => (theme::weighted(18.0, theme::SEMIBOLD), 24.0, theme::FG()),
        Kind::Heading(_) => (theme::weighted(15.0, theme::SEMIBOLD), 21.0, theme::FG()),
        Kind::Code => (theme::mono(12.0), 18.0, theme::FG()),
        Kind::Quote => (theme::font(BODY), 21.0, theme::FG_60()),
        Kind::Task(true) => (theme::font(BODY), 21.0, theme::FG_40()),
        _ => (theme::font(BODY), 21.0, theme::FG()),
    }
}

fn format_for(block: Block, style: &Style) -> TextFormat {
    let (mut font, line_height, mut color) = base_format(block);
    let bold = style.marks & BOLD != 0;
    if bold && matches!(block.kind, Kind::Paragraph | Kind::Bullet | Kind::Ordered | Kind::Task(_) | Kind::Quote) {
        font = theme::weighted(font.size, theme::BOLD);
    }
    let mut format = TextFormat { font_id: font, line_height: Some(line_height), color, ..Default::default() };
    if style.marks & CODE != 0 && block.kind != Kind::Code {
        format.font_id = theme::mono(12.0);
        format.background = theme::KEY_BG();
        format.expand_bg = 1.0;
    }
    if style.link.is_some() {
        color = theme::RED();
        format.color = color;
        format.underline = Stroke::new(1.0, color);
    }
    format.italics = style.marks & ITALIC != 0;
    if style.marks & UNDERLINE != 0 {
        format.underline = Stroke::new(1.0, color);
    }
    if style.marks & STRIKE != 0 || block.kind == Kind::Task(true) {
        format.strikethrough = Stroke::new(1.0, color);
    }
    format
}

pub fn layout(ui: &Ui, doc: &Doc, wrap_width: f32) -> Arc<Galley> {
    let mut whole = LayoutJob::default();
    whole.wrap.max_width = wrap_width;
    let mut galleys = Vec::with_capacity(doc.blocks.len());
    let mut chars = doc.styles.iter();
    let mut prev = None;
    for (line, text) in doc.text.split('\n').enumerate() {
        let block = doc.blocks[line];
        let indent = indent_x(block);
        let mut job = LayoutJob::default();
        job.wrap.max_width = (wrap_width - indent).max(20.0);
        let mut run_start = 0;
        let mut run_style: Option<&Style> = None;
        let mut byte = 0;
        for c in text.chars() {
            let style = chars.next().expect("one style per char");
            if run_style != Some(style) {
                if let Some(s) = run_style {
                    job.append(&text[run_start..byte], 0.0, format_for(block, s));
                }
                run_start = byte;
                run_style = Some(style);
            }
            byte += c.len_utf8();
        }
        match run_style {
            Some(s) => job.append(&text[run_start..], 0.0, format_for(block, s)),
            None => job.append("", 0.0, format_for(block, &Style::default())),
        }
        chars.next();
        let offset = whole.text.len();
        for section in &job.sections {
            let mut section = section.clone();
            section.byte_range = section.byte_range.start + offset..section.byte_range.end + offset;
            whole.sections.push(section);
        }
        whole.text.push_str(text);
        whole.text.push('\n');
        let laid = ui.ctx().fonts_mut(|f| f.layout_job(job));
        let shift = vec2(indent, gap_before(prev, block));
        let mut moved = (*laid).clone();
        for row in &mut moved.rows {
            row.pos += shift;
        }
        moved.rect = moved.rect.translate(shift);
        moved.mesh_bounds = moved.mesh_bounds.translate(shift);
        galleys.push(Arc::new(moved));
        prev = Some(block);
    }
    whole.text.pop();
    let mut galley = Galley::concat(Arc::new(whole), &galleys, ui.ctx().pixels_per_point());
    if prev.is_some_and(|b| b.kind == Kind::Code) {
        galley.rect.max.y += CODE_PAD;
    }
    Arc::new(galley)
}

fn line_geometry(galley: &Galley, origin: egui::Pos2, lines: usize) -> Vec<LineGeometry> {
    let mut out: Vec<LineGeometry> = Vec::with_capacity(lines);
    let mut current: Option<LineGeometry> = None;
    for row in &galley.rows {
        let rect = row.rect().translate(origin.to_vec2());
        match &mut current {
            Some(line) => line.rows = line.rows.union(rect),
            None => current = Some(LineGeometry { rows: rect, first_row: rect }),
        }
        if row.ends_with_newline {
            out.extend(current.take());
        }
    }
    out.extend(current);
    out
}

fn open_link(url: &str) {
    use windows::core::{w, HSTRING, PCWSTR};
    use windows::Win32::UI::Shell::ShellExecuteW;
    use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
    if url.starts_with("https://") || url.starts_with("http://") || url.starts_with("mailto:") {
        unsafe { ShellExecuteW(None, w!("open"), &HSTRING::from(url), PCWSTR::null(), PCWSTR::null(), SW_SHOWNORMAL) };
    }
}

impl NoteEditor {
    pub fn new(id: egui::Id, markdown: &str) -> Self {
        Self { doc: Doc::from_markdown(markdown), undo: Vec::new(), redo: Vec::new(), pending: None, pending_at: 0, last_push: f64::NEG_INFINITY, id }
    }

    pub fn load(&mut self, ctx: &egui::Context, markdown: &str) {
        self.doc = Doc::from_markdown(markdown);
        self.undo.clear();
        self.redo.clear();
        self.pending = None;
        self.set_cursor(ctx, 0, 0);
    }

    pub fn markdown(&self) -> String {
        self.doc.to_markdown()
    }

    pub fn focus(&self, ctx: &egui::Context) {
        ctx.memory_mut(|m| m.request_focus(self.id));
    }

    fn selection(&self, ctx: &egui::Context) -> (usize, usize) {
        let len = self.doc.styles.len();
        TextEditState::load(ctx, self.id)
            .and_then(|s| s.cursor.char_range())
            .map(|r| {
                let range = r.as_sorted_char_range();
                (range.start.0.min(len), range.end.0.min(len))
            })
            .unwrap_or((len, len))
    }

    fn set_cursor(&self, ctx: &egui::Context, start: usize, end: usize) {
        let mut state = TextEditState::load(ctx, self.id).unwrap_or_default();
        state.cursor.set_char_range(Some(CCursorRange::two(CCursor::new(start), CCursor::new(end))));
        state.store(ctx, self.id);
    }

    fn push_undo(&mut self, cursor: usize) {
        self.undo.push(Snapshot { doc: self.doc.clone(), cursor });
        if self.undo.len() > UNDO_LIMIT {
            self.undo.remove(0);
        }
        self.redo.clear();
    }

    fn restore(&mut self, ctx: &egui::Context, redo: bool) {
        let (from, to) = if redo { (&mut self.redo, &mut self.undo) } else { (&mut self.undo, &mut self.redo) };
        let Some(snapshot) = from.pop() else { return };
        let cursor = self.doc.styles.len().min(snapshot.cursor);
        to.push(Snapshot { doc: std::mem::replace(&mut self.doc, snapshot.doc), cursor });
        self.last_push = f64::NEG_INFINITY;
        self.set_cursor(ctx, snapshot.cursor, snapshot.cursor);
    }

    pub fn apply(&mut self, ctx: &egui::Context, format: Format) {
        let (start, end) = self.selection(ctx);
        self.push_undo(start);
        self.last_push = f64::NEG_INFINITY;
        match format {
            Format::Mark(mark) if start == end => {
                let mut style = self.pending.take().unwrap_or_else(|| start.checked_sub(1).filter(|&i| !self.doc.text.chars().nth(i).is_some_and(|c| c == '\n')).map(|i| self.doc.styles[i].clone()).unwrap_or_default());
                style.marks ^= mark;
                if mark == CODE && style.marks & CODE != 0 {
                    style.marks = CODE;
                }
                self.pending = Some(style);
                self.pending_at = start;
                self.undo.pop();
            }
            Format::Mark(mark) => self.doc.toggle_mark(start, end, mark),
            Format::Block(kind) => {
                let lines = self.doc.line_of(start)..=self.doc.line_of(end);
                self.doc.set_kind(lines, kind);
            }
        }
        self.focus(ctx);
    }

    fn handle_keys(&mut self, ui: &Ui) -> Option<Removal> {
        let ctx = ui.ctx().clone();
        let command = |key| KeyboardShortcut::new(Modifiers::COMMAND, key);
        let consume = |shortcut: KeyboardShortcut| ui.input_mut(|i| i.consume_shortcut(&shortcut));
        if consume(KeyboardShortcut::new(Modifiers::COMMAND.plus(Modifiers::SHIFT), Key::Z)) || consume(command(Key::Y)) {
            self.restore(&ctx, true);
        } else if consume(command(Key::Z)) {
            self.restore(&ctx, false);
        }
        for (format, shortcut) in FORMAT_SHORTCUTS {
            if consume(shortcut) {
                self.apply(&ctx, format);
            }
        }
        let outdent = consume(KeyboardShortcut::new(Modifiers::SHIFT, Key::Tab));
        if outdent || consume(KeyboardShortcut::new(Modifiers::NONE, Key::Tab)) {
            let (start, end) = self.selection(&ctx);
            let before = self.doc.clone();
            if self.doc.indent(self.doc.line_of(start)..=self.doc.line_of(end), if outdent { -1 } else { 1 }) {
                self.undo.push(Snapshot { doc: before, cursor: start });
                self.redo.clear();
            }
        }
        if consume(KeyboardShortcut::new(Modifiers::SHIFT, Key::Enter)) {
            let (start, end) = self.selection(&ctx);
            self.push_undo(start);
            self.doc.replace(start, end, "\n", None);
            let line = self.doc.line_of(start) + 1;
            let above = self.doc.blocks[line - 1];
            self.doc.blocks[line] = Block { kind: if above.kind == Kind::Task(true) { above.kind } else { self.doc.blocks[line].kind }, indent: above.indent, joined: above.kind != Kind::Code };
            self.set_cursor(&ctx, start + 1, start + 1);
        }
        ui.input(|i| {
            if i.key_pressed(Key::Backspace) {
                Some(Removal::Backward)
            } else if i.key_pressed(Key::Delete) {
                Some(Removal::Forward)
            } else {
                None
            }
        })
    }

    pub fn show(&mut self, ui: &mut Ui) -> bool {
        let ctx = ui.ctx().clone();
        let focused = ctx.memory(|m| m.has_focus(self.id));
        let before = self.doc.clone();
        let removal = if focused { self.handle_keys(ui) } else { None };
        let mut changed = self.doc != before;
        let background = ui.painter().add(Shape::Noop);
        let mut text = self.doc.text.clone();
        let doc = &self.doc;
        let pending = self.pending.clone();
        let id = self.id;
        let mut layouter = |ui: &Ui, buffer: &dyn TextBuffer, wrap_width: f32| {
            if buffer.as_str() == doc.text {
                layout(ui, doc, wrap_width)
            } else {
                let mut preview = doc.clone();
                let cursor = TextEditState::load(ui.ctx(), id).and_then(|s| s.cursor.char_range()).map_or(0, |r| r.primary.index.0);
                preview.sync(buffer.as_str(), cursor, pending.clone());
                layout(ui, &preview, wrap_width)
            }
        };
        let output = egui::TextEdit::multiline(&mut text)
            .id(self.id)
            .frame(egui::Frame::NONE)
            .margin(egui::Margin::ZERO)
            .desired_width(f32::INFINITY)
            .desired_rows(1)
            .lock_focus(true)
            .layouter(&mut layouter)
            .show(ui);
        let mut cursor = output.cursor_range.map_or(0, |r| r.primary.index.0);
        if text != self.doc.text {
            let now = ctx.input(|i| i.time);
            let snapshot = Snapshot { doc: self.doc.clone(), cursor };
            if let Some(edit) = self.doc.sync(&text, cursor, self.pending.clone()) {
                let boundary = edit.inserted.contains(char::is_whitespace) || !edit.removed.is_empty() && edit.inserted.is_empty() && self.undo.last().is_some_and(|s| s.doc.text.len() <= snapshot.doc.text.len());
                if now - self.last_push > 1.0 || boundary {
                    self.undo.push(snapshot);
                    if self.undo.len() > UNDO_LIMIT {
                        self.undo.remove(0);
                    }
                    self.redo.clear();
                }
                self.last_push = now;
                let mut moved = self.doc.handle_enter(&edit).or_else(|| removal.and_then(|r| self.doc.handle_backspace(&edit, r)));
                let mut dropped = 0;
                if moved.is_none() {
                    if let Some((at, mark)) = self.doc.handle_shortcuts(&edit) {
                        moved = Some(at);
                        dropped = mark;
                    }
                }
                if self.pending.is_some() {
                    self.pending_at = edit.start + edit.inserted.chars().count();
                }
                if let Some(at) = moved {
                    self.set_cursor(&ctx, at, at);
                    cursor = at;
                    if dropped != 0 {
                        let mut style = self.doc.styles[at - 1].clone();
                        style.marks &= !dropped;
                        self.pending = Some(style);
                        self.pending_at = at;
                    }
                }
            }
            changed = true;
        }
        if self.pending.is_some() && cursor != self.pending_at {
            self.pending = None;
        }
        self.decorate(ui, &output.galley, output.galley_pos, background, &output.response.response, &mut changed);
        changed
    }

    fn decorate(&mut self, ui: &Ui, galley: &Galley, origin: egui::Pos2, background: egui::layers::ShapeIdx, response: &egui::Response, changed: &mut bool) {
        if galley.rows.len() < self.doc.blocks.len() && galley.job.text != self.doc.text {
            return;
        }
        let lines = line_geometry(galley, origin, self.doc.blocks.len());
        let painter = ui.painter();
        let left = origin.x;
        let right = ui.max_rect().right();
        let mut shapes = Vec::new();
        let mut counters: Vec<u32> = Vec::new();
        let mut prev: Option<Block> = None;
        let pointer = ui.input(|i| i.pointer.interact_pos());
        let clicked = ui.input(|i| i.pointer.primary_clicked());
        let mut toggle = None;
        for (line, geometry) in lines.iter().enumerate() {
            let Some(&block) = self.doc.blocks.get(line) else { break };
            let x = left + indent_x(block);
            let center_y = geometry.first_row.center().y;
            if !block.kind.is_list() {
                counters.clear();
            }
            match block.kind {
                _ if block.joined => {}
                Kind::Bullet => {
                    let glyph = if block.indent % 3 == 1 { "◦" } else if block.indent % 3 == 2 { "▪" } else { "•" };
                    painter.text(pos2(x - 12.0, center_y), egui::Align2::CENTER_CENTER, glyph, theme::font(BODY), theme::FG());
                }
                Kind::Ordered => {
                    let level = block.indent as usize;
                    counters.resize(level + 1, 0);
                    let continues = prev.is_some_and(|p| p.kind.is_list() && (p.indent as usize > level || p.kind == Kind::Ordered && p.indent as usize == level));
                    counters[level] = if continues { counters[level] + 1 } else { 1 };
                    counters.truncate(level + 1);
                    painter.text(pos2(x - 6.0, center_y), egui::Align2::RIGHT_CENTER, format!("{}.", counters[level]), theme::font(BODY), theme::FG());
                }
                Kind::Task(checked) => {
                    let rect = Rect::from_center_size(pos2(x - 15.0, center_y), vec2(14.0, 14.0));
                    if checked {
                        painter.rect_filled(rect, 3.0, theme::ACCENT());
                        painter.add(Shape::line(vec![pos2(rect.left() + 3.0, rect.center().y), pos2(rect.left() + 6.0, rect.bottom() - 4.0), pos2(rect.right() - 3.0, rect.top() + 4.0)], Stroke::new(1.6, Color32::WHITE)));
                    } else {
                        painter.rect_stroke(rect, 3.0, Stroke::new(1.0, theme::FG_60()), egui::StrokeKind::Inside);
                    }
                    if clicked && pointer.is_some_and(|p| rect.expand(3.0).contains(p)) {
                        toggle = Some(line);
                    }
                }
                Kind::Quote => {
                    let rect = Rect::from_min_max(pos2(left, geometry.rows.top()), pos2(left + 2.0, geometry.rows.bottom()));
                    shapes.push(Shape::rect_filled(rect, 0.0, theme::FOOTER_DIVIDER()));
                }
                Kind::Code => {
                    let top = if prev.is_some_and(|p| p.kind == Kind::Code) { geometry.rows.top() } else { geometry.rows.top() - CODE_PAD };
                    let last = self.doc.blocks.get(line + 1).is_none_or(|b| b.kind != Kind::Code);
                    let bottom = if last { geometry.rows.bottom() + CODE_PAD } else { geometry.rows.bottom() };
                    let rect = Rect::from_min_max(pos2(left, top), pos2(right, bottom));
                    let rounding = egui::CornerRadius { nw: if top < geometry.rows.top() { 6 } else { 0 }, ne: if top < geometry.rows.top() { 6 } else { 0 }, sw: if last { 6 } else { 0 }, se: if last { 6 } else { 0 } };
                    shapes.push(Shape::rect_filled(rect, rounding, theme::KEY_BG()));
                }
                Kind::Rule => {
                    let rect = Rect::from_min_max(pos2(left, center_y - 1.0), pos2(right, center_y + 1.0));
                    shapes.push(Shape::rect_filled(rect, 0.0, theme::SEPARATOR()));
                }
                _ => {}
            }
            prev = Some(block);
        }
        painter.set(background, Shape::Vec(shapes));
        if let Some(line) = toggle {
            let (cursor, _) = self.selection(ui.ctx());
            self.push_undo(cursor);
            let block = &mut self.doc.blocks[line];
            block.kind = if block.kind == Kind::Task(true) { Kind::Task(false) } else { Kind::Task(true) };
            *changed = true;
        }
        let hovering_link = response.hovered()
            && pointer.is_some_and(|p| {
                let index = galley.cursor_from_pos(p - origin).index.0;
                ui.input(|i| i.modifiers.command) && self.doc.styles.get(index).is_some_and(|s| s.link.is_some())
            });
        if hovering_link {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        if response.clicked() && ui.input(|i| i.modifiers.command) {
            if let Some(p) = pointer {
                let index = galley.cursor_from_pos(p - origin).index.0;
                if let Some(url) = self.doc.styles.get(index).and_then(|s| s.link.clone()) {
                    open_link(&url);
                }
            }
        }
    }
}
