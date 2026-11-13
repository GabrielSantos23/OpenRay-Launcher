use super::editor::NoteEditor;
use super::Note;
use crate::theme;
use std::path::PathBuf;

const SAVE_DELAY: f64 = 0.4;

pub struct NotesWindow {
    data_dir: PathBuf,
    notes: Vec<Note>,
    current: usize,
    editor: NoteEditor,
    dirty_since: Option<f64>,
}

fn now_ms() -> f64 {
    crate::store::now_ms() as f64
}

impl NotesWindow {
    pub fn open(data_dir: PathBuf) -> Self {
        let mut notes = super::load(&data_dir);
        let current = match super::current(&notes) {
            Some(i) => i,
            None => {
                notes.push(super::new_note(now_ms()));
                notes.len() - 1
            }
        };
        notes[current].last_accessed_at = now_ms();
        let editor = NoteEditor::new(egui::Id::unique("note-editor"), &notes[current].markdown);
        Self { data_dir, notes, current, editor, dirty_since: None }
    }

    pub fn create(&mut self, ctx: &egui::Context) {
        let now = now_ms();
        let index = match self.notes.iter().position(|n| n.text.trim().is_empty()) {
            Some(i) => i,
            None => {
                self.notes.push(super::new_note(now));
                self.notes.len() - 1
            }
        };
        self.notes[index].last_accessed_at = now;
        self.current = index;
        self.editor.load(ctx, &self.notes[index].markdown);
        self.editor.focus(ctx);
        self.dirty_since = Some(f64::NEG_INFINITY);
    }

    fn flush(&mut self) {
        let note = &mut self.notes[self.current];
        note.markdown = self.editor.markdown();
        note.text = self.editor.doc.plain_text();
        note.updated_at = now_ms();
        if let Err(e) = super::save(&self.data_dir, &self.notes) {
            eprintln!("openray: could not save notes ({e})");
        }
        self.dirty_since = None;
    }

    pub fn ui(&mut self, ui: &mut egui::Ui) {
        let time = ui.input(|i| i.time);
        let frame = egui::Frame::NONE.fill(theme::BG).inner_margin(egui::Margin { left: 16, right: 16, top: 44, bottom: 16 });
        egui::CentralPanel::default().frame(frame).show(ui, |ui| {
            egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
                if self.editor.show(ui) {
                    self.dirty_since.get_or_insert(time);
                }
            });
        });
        if let Some(since) = self.dirty_since {
            if time - since >= SAVE_DELAY {
                self.flush();
            } else {
                ui.ctx().request_repaint_after(std::time::Duration::from_secs_f64(SAVE_DELAY));
            }
        }
        if ui.input(|i| i.viewport().close_requested()) && self.dirty_since.is_some() {
            self.flush();
        }
    }
}
