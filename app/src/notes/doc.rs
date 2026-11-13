use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag, TagEnd};
use std::sync::Arc;

pub const BOLD: u8 = 1;
pub const ITALIC: u8 = 2;
pub const UNDERLINE: u8 = 4;
pub const STRIKE: u8 = 8;
pub const CODE: u8 = 16;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Kind {
    #[default]
    Paragraph,
    Heading(u8),
    Bullet,
    Ordered,
    Task(bool),
    Quote,
    Code,
    Rule,
}

impl Kind {
    pub fn is_list(self) -> bool {
        matches!(self, Kind::Bullet | Kind::Ordered | Kind::Task(_))
    }

    fn continued(self) -> Kind {
        match self {
            Kind::Heading(_) | Kind::Rule => Kind::Paragraph,
            Kind::Task(_) => Kind::Task(false),
            other => other,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Block {
    pub kind: Kind,
    pub indent: u8,
    pub joined: bool,
}

#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct Style {
    pub marks: u8,
    pub link: Option<Arc<str>>,
}

#[derive(Clone, PartialEq, Debug)]
pub struct Doc {
    pub text: String,
    pub styles: Vec<Style>,
    pub blocks: Vec<Block>,
}

impl Default for Doc {
    fn default() -> Self {
        Self { text: String::new(), styles: Vec::new(), blocks: vec![Block::default()] }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Removal {
    Backward,
    Forward,
}

pub struct Edit {
    pub start: usize,
    pub removed: String,
    pub inserted: String,
    pub removed_blocks: Vec<Block>,
}

impl Doc {
    pub fn chars(&self) -> Vec<char> {
        self.text.chars().collect()
    }

    pub fn line_of(&self, char_index: usize) -> usize {
        self.text.chars().take(char_index).filter(|&c| c == '\n').count()
    }

    pub fn line_start(&self, line: usize) -> usize {
        if line == 0 {
            return 0;
        }
        let mut seen = 0;
        for (i, c) in self.text.chars().enumerate() {
            if c == '\n' {
                seen += 1;
                if seen == line {
                    return i + 1;
                }
            }
        }
        self.styles.len()
    }

    pub fn line_text(&self, line: usize) -> &str {
        self.text.split('\n').nth(line).unwrap_or("")
    }

    pub fn line_len(&self, line: usize) -> usize {
        self.line_text(line).chars().count()
    }

    pub fn plain_text(&self) -> String {
        self.text.split('\n').collect::<Vec<_>>().join("\n\n")
    }

    fn inherited(&self, start: usize, chars: &[char]) -> Style {
        let before = start.checked_sub(1).filter(|&i| chars[i] != '\n').map(|i| &self.styles[i]);
        let after = (start < chars.len() && chars[start] != '\n').then(|| &self.styles[start]);
        match (before, after) {
            (Some(b), a) => Style { marks: b.marks, link: b.link.clone().filter(|l| a.and_then(|a| a.link.as_ref()) == Some(l)) },
            (None, Some(a)) => Style { marks: a.marks, link: None },
            (None, None) => Style::default(),
        }
    }

    pub fn replace(&mut self, start: usize, end: usize, inserted: &str, style: Option<Style>) -> Vec<Block> {
        let chars = self.chars();
        let line = chars[..start].iter().filter(|&&c| c == '\n').count();
        let removed_lines = chars[start..end].iter().filter(|&&c| c == '\n').count();
        let style = style.unwrap_or_else(|| self.inherited(start, &chars));
        let at_line_end = chars.get(end).is_none_or(|&c| c == '\n');
        let mut text: String = chars[..start].iter().collect();
        text.push_str(inserted);
        text.extend(&chars[end..]);
        self.text = text;
        let new_styles = inserted.chars().map(|c| if c == '\n' { Style::default() } else { style.clone() });
        self.styles.splice(start..end, new_styles);
        let kept = self.blocks[line];
        let removed_blocks: Vec<Block> = self.blocks.drain(line + 1..=line + removed_lines).collect();
        let added = inserted.chars().filter(|&c| c == '\n').count();
        let mut next = Block { kind: if at_line_end { kept.kind.continued() } else { kept.kind }, indent: kept.indent, joined: false };
        if let Kind::Task(_) = next.kind {
            next.kind = Kind::Task(false);
        }
        self.blocks.splice(line + 1..line + 1, std::iter::repeat_n(next, added));
        removed_blocks
    }

    pub fn sync(&mut self, new_text: &str, cursor_after: usize, pending: Option<Style>) -> Option<Edit> {
        if new_text == self.text {
            return None;
        }
        let old: Vec<char> = self.chars();
        let new: Vec<char> = new_text.chars().collect();
        let prefix = old.iter().zip(&new).take_while(|(a, b)| a == b).count();
        let growth = new.len().saturating_sub(old.len());
        let start = prefix.min(cursor_after.saturating_sub(growth));
        let max_suffix = (old.len() - start).min(new.len() - start);
        let suffix = old.iter().rev().zip(new.iter().rev()).take(max_suffix).take_while(|(a, b)| a == b).count();
        let removed: String = old[start..old.len() - suffix].iter().collect();
        let inserted: String = new[start..new.len() - suffix].iter().collect();
        let removed_blocks = self.replace(start, old.len() - suffix, &inserted, pending);
        Some(Edit { start, removed, inserted, removed_blocks })
    }

    pub fn toggle_mark(&mut self, start: usize, end: usize, mark: u8) {
        let chars = self.chars();
        let targets: Vec<usize> = (start..end).filter(|&i| chars[i] != '\n').collect();
        let on = !targets.iter().all(|&i| self.styles[i].marks & mark != 0);
        for i in targets {
            let style = &mut self.styles[i];
            style.marks = if on { style.marks | mark } else { style.marks & !mark };
            if on && mark == CODE {
                style.marks = CODE;
                style.link = None;
            } else if on && style.marks & CODE != 0 {
                style.marks &= !CODE;
            }
        }
    }

    pub fn set_link(&mut self, start: usize, end: usize, url: Option<&str>) {
        let link: Option<Arc<str>> = url.map(Into::into);
        let chars = self.chars();
        for i in (start..end).filter(|&i| chars[i] != '\n') {
            self.styles[i].link = link.clone();
        }
    }

    pub fn set_kind(&mut self, lines: std::ops::RangeInclusive<usize>, kind: Kind) {
        let all = lines.clone().all(|l| self.blocks[l].kind == kind);
        for line in lines {
            let block = &mut self.blocks[line];
            block.kind = if all && kind != Kind::Paragraph { Kind::Paragraph } else { kind };
            if !block.kind.is_list() {
                block.indent = 0;
            }
        }
    }

    pub fn indent(&mut self, lines: std::ops::RangeInclusive<usize>, delta: i8) -> bool {
        let mut changed = false;
        for line in lines {
            let can_nest = line > 0 && self.blocks[line - 1].kind.is_list() && self.blocks[line - 1].indent >= self.blocks[line].indent;
            let block = &mut self.blocks[line];
            if !block.kind.is_list() || (delta > 0 && !can_nest) {
                continue;
            }
            let next = (block.indent as i8 + delta).clamp(0, 8) as u8;
            changed |= next != block.indent;
            block.indent = next;
        }
        changed
    }

    pub fn handle_enter(&mut self, edit: &Edit) -> Option<usize> {
        if edit.inserted != "\n" || !edit.removed.is_empty() {
            return None;
        }
        let line = self.line_of(edit.start);
        let block = self.blocks[line];
        let text = self.line_text(line).to_owned();
        if text.is_empty() && (block.kind.is_list() || matches!(block.kind, Kind::Quote | Kind::Code)) {
            self.replace(edit.start, edit.start + 1, "", None);
            let block = &mut self.blocks[line];
            if block.indent > 0 {
                block.indent -= 1;
            } else {
                block.kind = Kind::Paragraph;
            }
            return Some(edit.start);
        }
        if block.kind == Kind::Paragraph && text.starts_with("```") && self.line_len(line) == text.chars().count() && self.line_start(line) + text.chars().count() == edit.start {
            let begin = self.line_start(line);
            self.replace(begin, edit.start + 1, "", None);
            self.blocks[line].kind = Kind::Code;
            return Some(begin);
        }
        None
    }

    pub fn handle_backspace(&mut self, edit: &Edit, removal: Removal) -> Option<usize> {
        if edit.removed != "\n" || !edit.inserted.is_empty() || removal != Removal::Backward {
            return None;
        }
        self.replace(edit.start, edit.start, "\n", Some(Style::default()));
        let line = self.line_of(edit.start) + 1;
        self.blocks[line] = edit.removed_blocks[0];
        let block = &mut self.blocks[line];
        if block.indent > 0 {
            block.indent -= 1;
        } else if block.kind != Kind::Paragraph {
            block.kind = Kind::Paragraph;
        } else if block.joined {
            block.joined = false;
        } else {
            self.replace(edit.start, edit.start + 1, "", None);
            return Some(edit.start);
        }
        self.blocks[line].joined = false;
        Some(edit.start + 1)
    }

    pub fn handle_shortcuts(&mut self, edit: &Edit) -> Option<(usize, u8)> {
        let Some(c) = edit.inserted.chars().last() else { return None };
        if !edit.removed.is_empty() || edit.inserted.contains('\n') {
            return None;
        }
        let typed_end = edit.start + edit.inserted.chars().count();
        let line = self.line_of(edit.start);
        let begin = self.line_start(line);
        let before: String = self.text.chars().skip(begin).take(typed_end - begin).collect();
        let edit = Edit { start: typed_end - 1, removed: String::new(), inserted: c.to_string(), removed_blocks: Vec::new() };
        let block = self.blocks[line];
        if c == ' ' && block.kind == Kind::Paragraph && !block.joined || c == ' ' && block.kind == Kind::Bullet {
            let kind = match before.as_str() {
                "# " => Some(Kind::Heading(1)),
                "## " => Some(Kind::Heading(2)),
                "### " => Some(Kind::Heading(3)),
                "- " | "* " | "+ " => Some(Kind::Bullet),
                "> " => Some(Kind::Quote),
                "[ ] " | "[] " => Some(Kind::Task(false)),
                "[x] " | "[X] " => Some(Kind::Task(true)),
                s if s.len() > 2 && s.ends_with(". ") && s[..s.len() - 2].bytes().all(|b| b.is_ascii_digit()) => Some(Kind::Ordered),
                _ => None,
            };
            if let Some(kind) = kind.filter(|k| block.kind == Kind::Paragraph || matches!(k, Kind::Task(_))) {
                self.replace(begin, edit.start + 1, "", None);
                self.blocks[line].kind = kind;
                return Some((begin, 0));
            }
        }
        if c == '-' && before == "---" && block.kind == Kind::Paragraph && self.line_len(line) == 3 {
            self.replace(begin, begin + 3, "\n", Some(Style::default()));
            self.blocks[line] = Block { kind: Kind::Rule, ..Block::default() };
            self.blocks[line + 1] = Block::default();
            return Some((begin + 1, 0));
        }
        if c == ' ' {
            return self.autolink(begin, edit.start).map(|at| (at, 0));
        }
        if block.kind == Kind::Code {
            return None;
        }
        for (delimiter, mark) in [("**", BOLD), ("~~", STRIKE), ("++", UNDERLINE), ("`", CODE), ("*", ITALIC), ("_", ITALIC)] {
            if let Some(cursor) = self.inline_rule(&before, begin, delimiter, mark) {
                return Some((cursor, mark));
            }
        }
        None
    }

    fn inline_rule(&mut self, before: &str, begin: usize, delimiter: &str, mark: u8) -> Option<usize> {
        let body = before.strip_suffix(delimiter)?;
        if delimiter.len() == 1 && body.ends_with(delimiter) {
            return None;
        }
        let open = body.rfind(delimiter)?;
        let inner = &body[open + delimiter.len()..];
        if inner.is_empty() || inner.starts_with(' ') || inner.ends_with(' ') || (delimiter.len() == 1 && body[..open].ends_with(delimiter)) {
            return None;
        }
        let d = delimiter.chars().count();
        let open_char = begin + body[..open].chars().count();
        let inner_len = inner.chars().count();
        let close_char = open_char + d + inner_len;
        self.replace(close_char, close_char + d, "", None);
        self.replace(open_char, open_char + d, "", None);
        self.toggle_mark(open_char, open_char + inner_len, mark);
        if self.styles[open_char].marks & mark == 0 {
            self.toggle_mark(open_char, open_char + inner_len, mark);
        }
        Some(open_char + inner_len)
    }

    fn autolink(&mut self, begin: usize, space: usize) -> Option<usize> {
        let chars = self.chars();
        let word_start = (begin..space).rev().find(|&i| chars[i].is_whitespace()).map_or(begin, |i| i + 1);
        let word: String = chars[word_start..space].iter().collect();
        let url = if word.starts_with("https://") || word.starts_with("http://") {
            word.clone()
        } else if word.starts_with("www.") && word.len() > 4 {
            format!("https://{word}")
        } else {
            return None;
        };
        if self.styles[word_start].link.is_some() || self.styles[word_start].marks & CODE != 0 {
            return None;
        }
        self.set_link(word_start, space, Some(&url));
        None
    }

    pub fn from_markdown(markdown: &str) -> Doc {
        let mut builder = Builder::default();
        let parser = Parser::new_ext(markdown, Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TASKLISTS);
        for event in parser {
            builder.event(event);
        }
        builder.finish()
    }

    pub fn to_markdown(&self) -> String {
        let lines: Vec<&str> = self.text.split('\n').collect();
        let styles = self.line_styles();
        let mut out = String::new();
        let mut widths: Vec<usize> = Vec::new();
        let mut counters: Vec<u64> = Vec::new();
        let mut kinds: Vec<Kind> = Vec::new();
        let mut prev: Option<Block> = None;
        let mut prev_empty_paragraph = false;
        for (i, line) in lines.iter().enumerate() {
            let block = self.blocks[i];
            let list_prefix = |widths: &[usize], level: usize| " ".repeat(widths.iter().take(level).sum());
            if let Some(p) = prev {
                if p.kind == Kind::Code && block.kind != Kind::Code {
                    out.push_str("\n```");
                }
                if block.joined && p.kind != Kind::Code && p.kind != Kind::Rule {
                    out.push_str("  \n");
                    if block.kind.is_list() {
                        out.push_str(&list_prefix(&widths, block.indent as usize + 1));
                    } else if block.kind == Kind::Quote {
                        out.push_str("> ");
                    }
                    out.push_str(&inline_markdown(line, &styles[i], false));
                    prev = Some(block);
                    continue;
                }
                out.push_str(match (p.kind, block.kind) {
                    (Kind::Code, Kind::Code) => "\n",
                    (a, b) if a.is_list() && b.is_list() => "\n",
                    (Kind::Quote, Kind::Quote) => "\n>\n",
                    _ => "\n\n",
                });
            }
            if !block.kind.is_list() {
                widths.clear();
                counters.clear();
                kinds.clear();
            }
            match block.kind {
                Kind::Paragraph if line.is_empty() => {
                    if prev_empty_paragraph || i + 1 < lines.len() {
                        out.push_str("&nbsp;");
                    }
                }
                Kind::Paragraph => out.push_str(&inline_markdown(line, &styles[i], true)),
                Kind::Heading(level) => {
                    out.push_str(&"#".repeat(level as usize));
                    out.push(' ');
                    out.push_str(&inline_markdown(line, &styles[i], false));
                }
                Kind::Quote => {
                    out.push_str("> ");
                    out.push_str(&inline_markdown(line, &styles[i], false));
                }
                Kind::Rule => out.push_str("---"),
                Kind::Code => {
                    if prev.is_none_or(|p| p.kind != Kind::Code) {
                        out.push_str("```\n");
                    }
                    out.push_str(line);
                }
                Kind::Bullet | Kind::Ordered | Kind::Task(_) => {
                    let level = block.indent as usize;
                    widths.resize(level.max(widths.len()), 2);
                    widths.truncate(level);
                    counters.resize(level + 1, 0);
                    kinds.resize(level + 1, block.kind);
                    let continues = prev.is_some_and(|p| p.kind.is_list() && p.indent as usize >= level) && kinds[level] == block.kind && counters[level] > 0;
                    counters[level] = if continues { counters[level] + 1 } else { 1 };
                    kinds[level] = block.kind;
                    counters.truncate(level + 1);
                    kinds.truncate(level + 1);
                    let marker = match block.kind {
                        Kind::Ordered => format!("{}. ", counters[level]),
                        Kind::Task(true) => "- [x] ".to_string(),
                        Kind::Task(false) => "- [ ] ".to_string(),
                        _ => "- ".to_string(),
                    };
                    out.push_str(&list_prefix(&widths, level));
                    widths.push(if block.kind == Kind::Ordered { marker.len() } else { 2 });
                    out.push_str(&marker);
                    out.push_str(&inline_markdown(line, &styles[i], false));
                }
            }
            prev_empty_paragraph = block.kind == Kind::Paragraph && line.is_empty();
            prev = Some(block);
        }
        if prev.is_some_and(|p| p.kind == Kind::Code) {
            out.push_str("\n```");
        }
        out
    }

    fn line_styles(&self) -> Vec<Vec<Style>> {
        let mut lines = vec![Vec::new()];
        for (c, style) in self.text.chars().zip(&self.styles) {
            if c == '\n' {
                lines.push(Vec::new());
            } else {
                lines.last_mut().unwrap().push(style.clone());
            }
        }
        lines
    }
}

fn escape(text: &str, line_start: bool) -> String {
    let mut out = String::new();
    for (i, c) in text.chars().enumerate() {
        if matches!(c, '\\' | '*' | '_' | '`' | '~' | '[' | ']' | '<' | '&') || (c == '+' && text[text.char_indices().nth(i).map_or(0, |(b, _)| b)..].starts_with("++")) {
            out.push('\\');
        }
        out.push(c);
    }
    if line_start {
        let trimmed = out.trim_start_matches(' ');
        let head = &out[..out.len() - trimmed.len()];
        let digits = trimmed.bytes().take_while(u8::is_ascii_digit).count();
        let needs = trimmed.starts_with(['#', '>', '-', '+', '=']) || (digits > 0 && trimmed[digits..].starts_with(['.', ')']));
        if needs {
            let at = head.len() + if digits > 0 { digits } else { 0 };
            out.insert(at, '\\');
        }
    }
    out
}

fn markers(marks: u8) -> Vec<(u8, &'static str)> {
    [(BOLD, "**"), (ITALIC, "*"), (STRIKE, "~~"), (UNDERLINE, "++")].into_iter().filter(|(m, _)| marks & m != 0).collect()
}

fn inline_markdown(text: &str, styles: &[Style], line_start: bool) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut runs: Vec<(Style, String)> = Vec::new();
    for (i, c) in chars.iter().enumerate() {
        match runs.last_mut() {
            Some((style, run)) if *style == styles[i] => run.push(*c),
            _ => runs.push((styles[i].clone(), c.to_string())),
        }
    }
    let mut out = String::new();
    let mut open: Vec<(u8, &str)> = Vec::new();
    let mut link: Option<Arc<str>> = None;
    let mut pending_space = String::new();
    let close_all = |out: &mut String, open: &mut Vec<(u8, &str)>, link: &mut Option<Arc<str>>| {
        while let Some((_, m)) = open.pop() {
            out.push_str(m);
        }
        if let Some(url) = link.take() {
            out.push_str(&format!("]({url})"));
        }
    };
    for (index, (style, run)) in runs.iter().enumerate() {
        if style.marks & CODE != 0 {
            close_all(&mut out, &mut open, &mut link);
            out.push_str(&pending_space);
            pending_space.clear();
            let fence = if run.contains('`') { "``" } else { "`" };
            let pad = if run.contains('`') { " " } else { "" };
            if let Some(url) = &style.link {
                out.push('[');
                out.push_str(&format!("{fence}{pad}{run}{pad}{fence}]({url})"));
            } else {
                out.push_str(&format!("{fence}{pad}{run}{pad}{fence}"));
            }
            continue;
        }
        let trimmed_start = run.trim_start();
        let lead = &run[..run.len() - trimmed_start.len()];
        let body = trimmed_start.trim_end();
        let trail = &trimmed_start[body.len()..];
        if body.is_empty() {
            pending_space.push_str(run);
            continue;
        }
        if link != style.link {
            close_all(&mut out, &mut open, &mut link);
        }
        let wanted = markers(style.marks);
        if let Some(keep) = open.iter().position(|o| !wanted.contains(o)) {
            while open.len() > keep {
                out.push_str(open.pop().unwrap().1);
            }
        }
        out.push_str(&pending_space);
        out.push_str(lead);
        pending_space.clear();
        if link.is_none() {
            if let Some(url) = &style.link {
                out.push('[');
                link = Some(url.clone());
            }
        }
        for m in wanted {
            if !open.contains(&m) {
                out.push_str(m.1);
                open.push(m);
            }
        }
        out.push_str(&escape(body, line_start && index == 0 && lead.is_empty()));
        pending_space.push_str(trail);
    }
    close_all(&mut out, &mut open, &mut link);
    out.push_str(&pending_space);
    out
}

#[derive(Default)]
struct Builder {
    doc: Option<Doc>,
    lists: Vec<Option<u64>>,
    item_needs_block: bool,
    in_item_paragraph: bool,
    quote: usize,
    marks: u8,
    links: Vec<Arc<str>>,
    code_block: bool,
    pending_kind: Option<Kind>,
    heading: Option<u8>,
}

impl Builder {
    fn doc(&mut self) -> &mut Doc {
        self.doc.get_or_insert_with(Doc::default)
    }

    fn list_kind(&self) -> Kind {
        match self.lists.last() {
            Some(Some(_)) => Kind::Ordered,
            _ => Kind::Bullet,
        }
    }

    fn push_block(&mut self, kind: Kind, joined: bool) {
        let indent = self.lists.len().saturating_sub(1) as u8;
        let indent = if kind.is_list() { indent } else { 0 };
        let block = Block { kind, indent, joined };
        match &mut self.doc {
            None => {
                let mut doc = Doc::default();
                doc.blocks[0] = block;
                self.doc = Some(doc);
            }
            Some(doc) => {
                doc.text.push('\n');
                doc.styles.push(Style::default());
                doc.blocks.push(block);
            }
        }
    }

    fn ensure_item_block(&mut self) {
        if self.item_needs_block {
            self.item_needs_block = false;
            let kind = self.pending_kind.take().unwrap_or_else(|| self.list_kind());
            self.push_block(kind, false);
        }
    }

    fn text(&mut self, text: &str, extra: u8) {
        self.ensure_item_block();
        let link = self.links.last().cloned();
        let mut marks = self.marks | extra;
        let mut rest = text;
        while !rest.is_empty() {
            let toggle = rest.find("++").filter(|&at| marks & UNDERLINE != 0 || rest[at + 2..].contains("++"));
            let (head, tail) = match toggle {
                Some(at) => (&rest[..at], &rest[at + 2..]),
                None => (rest, ""),
            };
            for c in head.chars() {
                let c = if c == '\u{a0}' { ' ' } else { c };
                let style = Style { marks, link: link.clone() };
                let doc = self.doc();
                doc.text.push(c);
                doc.styles.push(style);
            }
            if toggle.is_some() {
                marks ^= UNDERLINE;
                self.marks ^= UNDERLINE;
            }
            rest = tail;
        }
    }

    fn event(&mut self, event: Event) {
        match event {
            Event::Start(Tag::Paragraph) => {
                if self.item_needs_block {
                    self.ensure_item_block();
                } else if !self.lists.is_empty() && self.in_item_paragraph {
                    let kind = self.list_kind();
                    self.push_block(kind, true);
                } else if self.quote > 0 {
                    self.push_block(Kind::Quote, false);
                } else if self.lists.is_empty() {
                    self.push_block(Kind::Paragraph, false);
                }
                self.in_item_paragraph = !self.lists.is_empty();
            }
            Event::Start(Tag::Heading { level, .. }) => {
                let level = match level {
                    HeadingLevel::H1 => 1,
                    HeadingLevel::H2 => 2,
                    _ => 3,
                };
                self.heading = Some(level);
                self.push_block(Kind::Heading(level), false);
            }
            Event::End(TagEnd::Heading(_)) => self.heading = None,
            Event::Start(Tag::BlockQuote(_)) => self.quote += 1,
            Event::End(TagEnd::BlockQuote(_)) => self.quote = self.quote.saturating_sub(1),
            Event::Start(Tag::List(start)) => {
                self.ensure_item_block();
                self.lists.push(start);
            }
            Event::End(TagEnd::List(_)) => {
                self.lists.pop();
                self.item_needs_block = false;
            }
            Event::Start(Tag::Item) => {
                self.item_needs_block = true;
                self.in_item_paragraph = false;
                self.pending_kind = None;
            }
            Event::End(TagEnd::Item) => {
                self.ensure_item_block();
                self.in_item_paragraph = false;
            }
            Event::TaskListMarker(checked) => {
                if self.item_needs_block {
                    self.pending_kind = Some(Kind::Task(checked));
                    self.ensure_item_block();
                } else if let Some(doc) = &mut self.doc {
                    doc.blocks.last_mut().unwrap().kind = Kind::Task(checked);
                }
            }
            Event::Start(Tag::CodeBlock(kind)) => {
                let _ = matches!(kind, CodeBlockKind::Fenced(_));
                self.code_block = true;
                self.push_block(Kind::Code, false);
            }
            Event::End(TagEnd::CodeBlock) => {
                self.code_block = false;
                let doc = self.doc();
                if doc.text.ends_with('\n') {
                    doc.text.pop();
                    doc.styles.pop();
                    doc.blocks.pop();
                }
            }
            Event::Text(text) if self.code_block => {
                for (i, line) in text.split('\n').enumerate() {
                    if i > 0 {
                        self.push_block(Kind::Code, false);
                    }
                    let doc = self.doc();
                    doc.text.push_str(line);
                    doc.styles.extend(line.chars().map(|_| Style::default()));
                }
            }
            Event::Text(text) => {
                if self.lists.is_empty() && self.quote == 0 && self.heading.is_none() && self.doc.is_none() {
                    self.push_block(Kind::Paragraph, false);
                }
                self.text(&text, 0);
            }
            Event::Code(text) => self.text(&text, CODE),
            Event::Html(text) | Event::InlineHtml(text) => self.text(&text, 0),
            Event::SoftBreak => self.text(" ", 0),
            Event::HardBreak => {
                let kind = self.doc().blocks.last().map_or(Kind::Paragraph, |b| b.kind);
                self.push_block(kind, true);
            }
            Event::Rule => self.push_block(Kind::Rule, false),
            Event::Start(Tag::Emphasis) => self.marks |= ITALIC,
            Event::End(TagEnd::Emphasis) => self.marks &= !ITALIC,
            Event::Start(Tag::Strong) => self.marks |= BOLD,
            Event::End(TagEnd::Strong) => self.marks &= !BOLD,
            Event::Start(Tag::Strikethrough) => self.marks |= STRIKE,
            Event::End(TagEnd::Strikethrough) => self.marks &= !STRIKE,
            Event::Start(Tag::Link { dest_url, .. }) => self.links.push(dest_url.as_ref().into()),
            Event::End(TagEnd::Link) => {
                self.links.pop();
            }
            _ => {}
        }
    }

    fn finish(self) -> Doc {
        let mut doc = self.doc.unwrap_or_default();
        if doc.blocks.len() == 1 && doc.blocks[0].kind == Kind::Paragraph && doc.text.trim().is_empty() {
            return Doc::default();
        }
        let lines: Vec<String> = doc.text.split('\n').map(str::to_owned).collect();
        for (i, line) in lines.iter().enumerate() {
            if doc.blocks[i].kind == Kind::Paragraph && !line.is_empty() && line.chars().all(|c| c == ' ') {
                let start = doc.line_start(i);
                doc.replace(start, start + line.chars().count(), "", Some(Style::default()));
            }
        }
        doc
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn round_trip(markdown: &str) -> String {
        Doc::from_markdown(markdown).to_markdown()
    }

    #[test]
    fn blocks_round_trip() {
        for md in [
            "# Title\n\nSome text",
            "## Two\n\n### Three",
            "- one\n- two\n  - nested\n- three",
            "1. a\n2. b\n3. c",
            "- [ ] todo\n- [x] done",
            "> quoted",
            "```\nlet x = 1;\nlet y = 2;\n```",
            "a\n\n---\n\nb",
            "1. a\n   - inside\n2. b",
        ] {
            assert_eq!(round_trip(md), md, "{md}");
        }
    }

    #[test]
    fn marks_round_trip() {
        for md in ["**bold** and *italic*", "~~gone~~ ++under++", "`code` here", "[site](https://example.com) after", "**bold *both* bold**", "a\\*b\\_c"] {
            assert_eq!(round_trip(md), md, "{md}");
        }
    }

    #[test]
    fn model_reflects_markdown() {
        let doc = Doc::from_markdown("# T\n\n- [x] done\n- **b**");
        assert_eq!(doc.text, "T\ndone\nb");
        assert_eq!(doc.blocks.iter().map(|b| b.kind).collect::<Vec<_>>(), [Kind::Heading(1), Kind::Task(true), Kind::Bullet]);
        assert_eq!(doc.styles[7].marks, BOLD);
        assert_eq!(doc.plain_text(), "T\n\ndone\n\nb");
    }

    #[test]
    fn empty_paragraphs_survive_as_nbsp() {
        let doc = Doc::from_markdown("a\n\n&nbsp;\n\n&nbsp;\n\nb");
        assert_eq!(doc.text, "a\n\n\nb");
        assert_eq!(doc.to_markdown(), "a\n\n&nbsp;\n\n&nbsp;\n\nb");
        assert_eq!(Doc::from_markdown(""), Doc::default());
    }

    #[test]
    fn hard_breaks_stay_in_the_block() {
        let doc = Doc::from_markdown("one  \ntwo");
        assert_eq!(doc.text, "one\ntwo");
        assert!(doc.blocks[1].joined);
        assert_eq!(doc.to_markdown(), "one  \ntwo");
    }

    #[test]
    fn line_start_markers_are_escaped() {
        let mut doc = Doc::default();
        doc.replace(0, 0, "# not a heading", None);
        assert_eq!(Doc::from_markdown(&doc.to_markdown()).blocks[0].kind, Kind::Paragraph);
        let mut doc = Doc::default();
        doc.replace(0, 0, "1. not a list", None);
        assert_eq!(Doc::from_markdown(&doc.to_markdown()).text, "1. not a list");
    }

    #[test]
    fn typing_inherits_style_and_enter_continues_lists() {
        let mut doc = Doc::from_markdown("- **ab**");
        doc.sync("abc", 3, None);
        assert_eq!(doc.styles[2].marks, BOLD);
        let edit = doc.sync("abc\n", 4, None).unwrap();
        assert_eq!(doc.handle_enter(&edit), None);
        assert_eq!(doc.blocks[1].kind, Kind::Bullet);
        let edit = doc.sync("abc\n\n", 5, None).unwrap();
        assert_eq!(doc.handle_enter(&edit), Some(4));
        assert_eq!(doc.text, "abc\n");
        assert_eq!(doc.blocks[1].kind, Kind::Paragraph);
    }

    #[test]
    fn enter_after_heading_starts_paragraph_and_splitting_keeps_kind() {
        let mut doc = Doc::from_markdown("# Title");
        doc.sync("Title\n", 6, None);
        assert_eq!(doc.blocks[1].kind, Kind::Paragraph);
        let mut doc = Doc::from_markdown("# Title");
        doc.sync("Ti\ntle", 3, None);
        assert_eq!(doc.blocks[1].kind, Kind::Heading(1));
    }

    #[test]
    fn backspace_at_list_start_lifts_before_merging() {
        let mut doc = Doc::from_markdown("a\n\n- b");
        let edit = doc.sync("ab", 1, None).unwrap();
        assert_eq!(doc.handle_backspace(&edit, Removal::Backward), Some(2));
        assert_eq!(doc.text, "a\nb");
        assert_eq!(doc.blocks[1].kind, Kind::Paragraph);
        let edit = doc.sync("ab", 1, None).unwrap();
        assert_eq!(doc.handle_backspace(&edit, Removal::Backward), Some(1));
        assert_eq!(doc.text, "ab");
        assert_eq!(doc.blocks.len(), 1);
    }

    #[test]
    fn markdown_shortcuts_convert_blocks() {
        for (typed, kind) in [("# ", Kind::Heading(1)), ("- ", Kind::Bullet), ("1. ", Kind::Ordered), ("[ ] ", Kind::Task(false)), ("> ", Kind::Quote)] {
            let mut doc = Doc::default();
            let mut edit = None;
            for (i, _) in typed.char_indices() {
                edit = doc.sync(&typed[..=i], i + 1, None);
            }
            assert_eq!(doc.handle_shortcuts(&edit.unwrap()), Some((0, 0)), "{typed}");
            assert_eq!((doc.text.as_str(), doc.blocks[0].kind), ("", kind));
        }
    }

    #[test]
    fn inline_shortcuts_apply_marks() {
        let mut doc = Doc::default();
        let text = "a **bold**";
        let mut edit = None;
        for (i, _) in text.char_indices() {
            edit = doc.sync(&text[..=i], i + 1, None);
            if i + 1 < text.len() {
                assert_eq!(doc.handle_shortcuts(edit.as_ref().unwrap()), None);
            }
        }
        assert_eq!(doc.handle_shortcuts(&edit.unwrap()), Some((6, BOLD)));
        assert_eq!(doc.text, "a bold");
        assert_eq!(doc.styles[2].marks, BOLD);
        assert_eq!(doc.styles[1].marks, 0);
    }

    #[test]
    fn rule_shortcut_and_code_fence() {
        let mut doc = Doc::default();
        doc.sync("--", 2, None);
        let edit = doc.sync("---", 3, None).unwrap();
        assert_eq!(doc.handle_shortcuts(&edit), Some((1, 0)));
        assert_eq!(doc.blocks.iter().map(|b| b.kind).collect::<Vec<_>>(), [Kind::Rule, Kind::Paragraph]);
        let mut doc = Doc::default();
        doc.sync("```", 3, None);
        let edit = doc.sync("```\n", 4, None).unwrap();
        assert_eq!(doc.handle_enter(&edit), Some(0));
        assert_eq!((doc.text.as_str(), doc.blocks[0].kind), ("", Kind::Code));
    }

    #[test]
    fn shortcuts_fire_when_several_chars_arrive_in_one_frame() {
        let mut doc = Doc::default();
        let edit = doc.sync("- ", 2, None).unwrap();
        assert_eq!(doc.handle_shortcuts(&edit), Some((0, 0)));
        assert_eq!(doc.blocks[0].kind, Kind::Bullet);
        let edit = doc.sync("x **b**", 7, None).unwrap();
        assert_eq!(doc.handle_shortcuts(&edit), Some((3, BOLD)));
        assert_eq!((doc.text.as_str(), doc.styles[2].marks), ("x b", BOLD));
    }

    #[test]
    fn ordered_numbering_restarts_after_another_list_kind() {
        assert_eq!(round_trip("- [ ] a
  - [ ] b

1. one
2. two"), "- [ ] a
  - [ ] b
1. one
2. two");
    }

    #[test]
    fn autolink_on_space() {
        let mut doc = Doc::default();
        doc.sync("see https://a.io", 16, None);
        let edit = doc.sync("see https://a.io ", 17, None).unwrap();
        doc.handle_shortcuts(&edit);
        assert_eq!(doc.styles[4].link.as_deref(), Some("https://a.io"));
        assert_eq!(doc.to_markdown(), "see [https://a.io](https://a.io) ");
    }

    #[test]
    fn toggles_and_indents() {
        let mut doc = Doc::from_markdown("- a\n- b");
        doc.toggle_mark(0, 3, ITALIC);
        assert_eq!(doc.styles[0].marks, ITALIC);
        assert_eq!(doc.styles[1], Style::default());
        doc.toggle_mark(0, 3, ITALIC);
        assert_eq!(doc.styles[0].marks, 0);
        assert!(doc.indent(1..=1, 1));
        assert!(!doc.indent(0..=0, 1));
        assert_eq!(doc.to_markdown(), "- a\n  - b");
        doc.set_kind(0..=1, Kind::Paragraph);
        assert_eq!(doc.to_markdown(), "a\n\nb");
    }

    #[test]
    fn diff_prefers_the_cursor_side_for_repeated_chars() {
        let mut doc = Doc::from_markdown("**a**a");
        let edit = doc.sync("aaa", 2, None).unwrap();
        assert_eq!(edit.start, 1);
        assert_eq!(doc.styles[1].marks, BOLD);
        assert_eq!(doc.styles[2].marks, 0);
    }
}
