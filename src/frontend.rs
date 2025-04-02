use std::{
    fmt,
    io::{self, Write},
    str,
};

use crossterm::{cursor, style, Command, QueueableCommand};

// TODO: Deal with width
pub struct Article {
    content: Vec<String>,
    first: usize,
    last: usize,
}

impl Article {
    pub fn new(content: Vec<Vec<String>>, height: usize) -> io::Result<Self> {
        Ok(Self {
            content: content.into_iter().flatten().collect(),
            first: 0,
            last: height,
        })
    }

    pub fn draw(&self, mut qc: impl QueueableCommand + Write) -> io::Result<()> {
        qc.queue(cursor::MoveTo(0, 0))?;
        for line in self.content.iter().take(self.last - self.first) {
            qc.write(line.as_bytes())?;
            qc.queue(cursor::MoveDown(1))?
                .queue(cursor::MoveToColumn(0))?;
        }
        Ok(())
    }

    // TODO: scroll_up
    // TODO: scroll_down
    // TODO: change_dimens
}

pub trait Component {
    // NOTE: To implement scrolling correctly you must not use cursor::Move commands
    // TODO: Add an assert command to check the above note
    fn build(text: &str) -> Result<Vec<String>, fmt::Error>;

    fn push(dest: &mut Vec<String>, src: impl Command) -> fmt::Result {
        let mut line = String::new();
        src.write_ansi(&mut line)?;
        dest.push(line);
        Ok(())
    }
}

// TODO: Deal with width
pub struct Paragraph;

impl Component for Paragraph {
    fn build(text: &str) -> Result<Vec<String>, fmt::Error> {
        const WIDTH: usize = 30;
        const IDENT: usize = 4;
        let mut res = vec![String::new()];
        let mut buf = String::from(" ".repeat(IDENT));
        for word in text.split_whitespace() {
            if buf.len() + word.len() >= WIDTH {
                buf.pop();
                Self::push(&mut res, style::Print(&buf)).unwrap();
                buf.clear();
            }
            buf.push_str(word);
            buf.push(' ');
        }
        Self::push(&mut res, style::Print(&buf.trim_end())).unwrap();
        Ok(res)
    }
}

#[cfg(test)]
mod paragraph_tests {
    use super::{Component, Paragraph};

    #[test]
    fn test_one_word_paragraph() {
        let right = vec!["", "    TEST"];
        assert_eq!(Paragraph::build("TEST").unwrap(), right);
    }

    #[test]
    fn test_empty_paragraph() {
        let right = vec!["", ""];
        assert_eq!(Paragraph::build("").unwrap(), right);
    }
}
