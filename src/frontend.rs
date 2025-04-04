use std::io::{self, Write};

use crossterm::{QueueableCommand, cursor, style::Stylize, terminal};

pub enum Components {
    Paragraph(String),
    Title(String),
}

pub fn build_article(components: Vec<Components>, width: usize) -> Vec<String> {
    components
        .iter()
        .flat_map(|comp| match comp {
            Components::Paragraph(text) => Paragraph::build(&text, width),
            Components::Title(text) => Title::build(&text, width),
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
        for line in self.content.iter().take(self.height as usize) {
            qc.write(line.as_bytes())?;
            qc.queue(cursor::MoveDown(1))?
                .queue(cursor::MoveToColumn(0))?;
        }
        Ok(())
    }

    pub fn scroll_down(&mut self, mut qc: impl QueueableCommand + Write) -> io::Result<()> {
        if self.first <= 0 {
            return Ok(());
        }
        self.first -= 1;
        qc.queue(terminal::ScrollDown(1))?
            .queue(cursor::MoveTo(0, 0))?;
        qc.write(self.content[self.first as usize].as_bytes())?;
        Ok(())
    }

    pub fn scroll_up(&mut self, mut qc: impl QueueableCommand + Write) -> io::Result<()> {
        if (self.first + self.height) as usize >= self.content.len() {
            return Ok(());
        }
        self.first += 1;
        qc.queue(terminal::ScrollUp(1))?
            .queue(cursor::MoveTo(0, self.height))?;
        qc.write(self.content[(self.first + self.height - 1) as usize].as_bytes())?;
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
        Paragraph::build(text, width)
            .iter()
            .map(|line| line.clone().bold().to_string())
            .collect()
    }
}

// TODO: Add the Lead component
// TODO: Add the List component
// TODO: Add the Boxed component

#[cfg(test)]
mod text_wrap_tests {
    use super::wrap_text;

    #[test]
    fn one_word() {
        let right = vec!["TEST"];
        assert_eq!(wrap_text("TEST", 10), right);
    }

    #[test]
    fn empty_line() {
        let right = vec![""];
        assert_eq!(wrap_text("", 10), right);
    }

    #[test]
    fn two_line() {
        let right = vec!["TEST", "TEST"];
        assert_eq!(wrap_text("TEST TEST", 5), right);
    }

    // TODO: Figure out how to warn if text couldn't wrap
    #[test]
    fn too_short() {
        let right = vec!["TEST", "TEST"];
        assert_eq!(wrap_text("TEST TEST", 3), right);
    }
}

// TODO: Write more tests
