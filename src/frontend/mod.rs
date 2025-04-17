use std::io::{self, Write};

use crossterm::{
    QueueableCommand, cursor,
    style::{self, Stylize},
    terminal,
};

use crate::{Direction, Feed, FeedItem, View};

pub enum Components {
    Title(String),
    Subtitle(String),
    Lead(String),
    Paragraph(String),
    Boxed(Vec<String>),
}

pub fn build_componenets(components: &[Components], width: usize) -> Vec<String> {
    components
        .iter()
        .flat_map(|comp| match comp {
            Components::Title(text) => Title::build(&text, width),
            Components::Subtitle(text) => Subtitle::build(&text, width),
            Components::Lead(text) => Lead::build(&text, width),
            Components::Paragraph(text) => Paragraph::build(&text, width),
            Components::Boxed(text) => Boxed::build(&text.join("\n"), width),
        })
        .collect()
}

pub struct TextPad {
    content: Vec<String>,
    first: u16,
    height: u16,
    width: u16,
}

impl TextPad {
    pub fn new(content: Vec<String>, height: u16, width: u16) -> io::Result<Self> {
        Ok(Self {
            content,
            first: 0,
            height,
            width,
        })
    }

    pub fn draw(&self, mut qc: impl QueueableCommand + Write) -> io::Result<()> {
        qc.queue(terminal::Clear(terminal::ClearType::All))?
            .queue(cursor::MoveTo(0, 0))?;
        for line in self
            .content
            .iter()
            .skip(self.first as usize)
            .take(self.height as usize)
        {
            qc.write(line.as_bytes())?;
            qc.queue(cursor::MoveDown(1))?
                .queue(cursor::MoveToColumn(0))?;
        }
        Ok(())
    }

    fn scroll_by_lines(
        &mut self,
        mut qc: impl QueueableCommand + Write,
        lines: i16,
    ) -> io::Result<()> {
        let mut draw_line = 0;
        let is_up = lines < 0;
        let mut lines = lines.abs() as u16;

        if is_up {
            if self.first < lines {
                lines = self.first;
            }
            self.first -= lines;

            qc.queue(terminal::ScrollDown(lines))?;
        } else {
            let last = self.first + self.height;
            if (self.content.len() as u16) < self.height {
                lines = 0
            } else if (last + lines) as usize >= self.content.len() {
                lines = self.content.len() as u16 - last;
            }
            self.first += lines;
            draw_line = self.height - lines;

            qc.queue(terminal::ScrollUp(lines))?;
        }

        qc.queue(cursor::MoveTo(0, draw_line))?;
        for line in self
            .content
            .iter()
            .skip((self.first + draw_line) as usize)
            .take(lines as usize)
        {
            qc.write(line.as_bytes())?;
            qc.queue(cursor::MoveDown(1))?
                .queue(cursor::MoveToColumn(0))?;
        }
        Ok(())
    }

    pub fn scroll_by(
        &mut self,
        mut qc: impl QueueableCommand + Write,
        dir: Direction,
        view: View,
    ) -> io::Result<()> {
        match (dir, view) {
            (Direction::Up, View::Article) => self.scroll_by_lines(&mut qc, -1)?,
            (Direction::Down, View::Article) => self.scroll_by_lines(&mut qc, 1)?,
            (Direction::Up, View::Feed) => {
                let lines = self
                    .content
                    .iter()
                    .take(self.first as usize)
                    .rev()
                    .take_while(|line| !line.chars().all(|c| c.is_whitespace()))
                    .count() as i16
                    + 1;
                self.scroll_by_lines(&mut qc, -lines)?;
            }
            (Direction::Down, View::Feed) => {
                let lines = self
                    .content
                    .iter()
                    .skip((self.first + self.height) as usize + 1)
                    .take_while(|line| !line.chars().all(|c| c.is_whitespace()))
                    .count() as i16
                    + 1;
                self.scroll_by_lines(&mut qc, lines)?;
            }
        };
        Ok(())
    }

    pub fn resize(&mut self, nw: u16, nh: u16) -> io::Result<()> {
        self.height = nh;
        self.width = nw;
        Ok(())
    }
}

fn wrap_text(text: &str, width: usize) -> Vec<String> {
    let mut res = Vec::new();
    let mut buf = String::new();
    for word in text.split(" ") {
        if buf.len() + word.len() >= width {
            buf.pop();
            res.push(buf.clone());
            buf.clear();
        }
        buf.push_str(word);
        buf.push(' ');
    }
    res.push(buf.trim_end().to_string());
    res
}

pub trait Component {
    // NOTE: To implement scrolling correctly you must not use cursor::Move commands
    // TODO: Add an assert command to check the above note
    fn build(text: &str, width: usize) -> Vec<String>;
}

// BUG: Doesn't display utf-8 properly
pub struct Paragraph;
impl Component for Paragraph {
    fn build(text: &str, width: usize) -> Vec<String> {
        const IDENT: usize = 4;
        let mut res = vec![String::new()];
        let text = " ".repeat(IDENT) + text.trim();
        res.append(&mut wrap_text(&text, width));
        res
    }
}

pub struct Title;
impl Component for Title {
    fn build(text: &str, width: usize) -> Vec<String> {
        let mut res = vec![String::new()];
        let wraped_text = wrap_text(text.trim(), width);
        let wraped_text = wraped_text.iter().map(|line| {
            let ident = (width - line.len()) / 2;
            let ident = " ".repeat(ident);
            // TODO: Style this better
            format!("{ident}{}{ident}", line.clone().on_dark_grey().bold())
        });
        res.extend(wraped_text);
        res
    }
}

pub struct Lead;
impl Component for Lead {
    fn build(text: &str, width: usize) -> Vec<String> {
        Paragraph::build(text, width)
            .iter()
            .map(|line| line.clone().bold().to_string())
            .collect()
    }
}

pub struct Subtitle;
impl Component for Subtitle {
    fn build(text: &str, width: usize) -> Vec<String> {
        let mut res = vec![String::new()];
        res.extend(
            Paragraph::build(text, width)
                .iter()
                .map(|line| line.clone().bold().to_string()),
        );
        res
    }
}

pub struct Boxed;
impl Component for Boxed {
    fn build(text: &str, width: usize) -> Vec<String> {
        let mut res = vec![" ".repeat(width).to_string()];
        res.push(format!(" ┌{}┐ ", "─".repeat(width - 4)));
        let mut text = text
            .split("\n")
            .flat_map(|p| Paragraph::build(p, width - 6))
            .map(|row| format!(" │ {row}{} │ ", " ".repeat(width - 6 - row.chars().count())));
        text.next();
        res.extend(text);
        res.push(format!(" └{}┘ ", "─".repeat(width - 4)));
        res
    }
}

impl FeedItem {
    pub fn build(&self) -> Components {
        let mut rows = vec![self.title.clone()];
        if let Some(dt) = self.published {
            rows.push(dt.to_string());
        }
        Components::Boxed(rows)
    }
}

impl Feed {
    pub fn build(&self) -> Vec<Components> {
        self.items.iter().map(|i| i.build()).collect()
    }

    pub fn set_positions(&mut self, content: &[String]) {
        let mut items = self.items.iter_mut();
        for (i, line) in content.iter().enumerate() {
            if line.chars().all(|c| c.is_whitespace()) {
                if let Some(item) = items.next() {
                    item.at = Some(i);
                }
            }
        }
    }

    pub fn redraw_selected(
        &self,
        mut qc: impl QueueableCommand + Write,
        textpad: &mut TextPad,
        is_selected: bool,
    ) -> io::Result<()> {
        let first_row = self.items[self.selected].at.unwrap();
        let last_row = {
            if self.selected + 1 >= self.items.len() {
                textpad.content.len()
            } else {
                self.items[self.selected + 1].at.unwrap()
            }
        };
        let item_content = &textpad.content[first_row..last_row];
        qc.queue(cursor::MoveTo(1, first_row as u16 - textpad.first))?;
        for line in item_content {
            if is_selected {
                qc.queue(style::PrintStyledContent(line.clone().red()))?;
            } else {
                qc.queue(style::Print(line))?;
            }
            qc.queue(cursor::MoveDown(1))?
                .queue(cursor::MoveToColumn(0))?;
        }
        Ok(())
    }

    pub fn select(
        &mut self,
        mut qc: impl QueueableCommand + Write,
        textpad: &mut TextPad,
        st: Direction,
    ) -> io::Result<()> {
        self.redraw_selected(&mut qc, textpad, false)?;
        match st {
            Direction::Up => {
                if self.selected > 0 {
                    self.selected -= 1;
                    if self.selected > 0 {
                        let next_row = self.items[self.selected - 1].at.unwrap() as u16;
                        if next_row < textpad.first {
                            textpad.scroll_by(&mut qc, st, View::Feed)?;
                        }
                    }
                }
            }
            Direction::Down => {
                if self.selected < self.items.len() - 1 {
                    self.selected += 1;
                    let next_row = if self.selected < self.items.len() - 2 {
                        self.items[self.selected + 2].at.unwrap() as u16
                    } else if self.selected < self.items.len() - 1 {
                        self.items[self.selected + 1].at.unwrap() as u16
                    } else {
                        textpad.first
                    };
                    if next_row - textpad.first >= textpad.height {
                        textpad.scroll_by(&mut qc, st, View::Feed)?;
                    }
                }
            }
        };
        self.redraw_selected(&mut qc, textpad, true)?;
        Ok(())
    }
}

// TODO: Add the List component

#[cfg(test)]
mod tests;
