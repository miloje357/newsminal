use std::{
    fmt,
    io::{self, Write},
};

use crossterm::{cursor, terminal, Command, QueueableCommand};

// TODO: Remove last after implementing dimens
pub struct Article {
    content: Vec<String>,
    first: u16,
    last: u16,
}

impl Article {
    pub fn new(content: Vec<Vec<String>>, height: u16) -> io::Result<Self> {
        Ok(Self {
            content: content.into_iter().flatten().collect(),
            first: 0,
            last: height - 1,
        })
    }

    pub fn draw(&self, mut qc: impl QueueableCommand + Write) -> io::Result<()> {
        qc.queue(cursor::MoveTo(0, 0))?;
        for line in self
            .content
            .iter()
            .take((self.last - self.first + 1).into())
        {
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
        self.last -= 1;
        qc.queue(terminal::ScrollDown(1))?
            .queue(cursor::MoveTo(0, 0))?;
        qc.write(self.content[self.first as usize].as_bytes())?;
        Ok(())
    }

    pub fn scroll_up(&mut self, mut qc: impl QueueableCommand + Write) -> io::Result<()> {
        if self.last as usize >= self.content.len() - 1 {
            return Ok(());
        }
        self.first += 1;
        self.last += 1;
        qc.queue(terminal::ScrollUp(1))?
            .queue(cursor::MoveTo(0, self.last - self.first))?;
        qc.write(self.content[self.last as usize].as_bytes())?;
        Ok(())
    }

    // TODO: resize
}

fn wrap_text(text: &str, width: usize) -> Result<Vec<String>, fmt::Error> {
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
    Ok(res)
}

pub trait Component {
    // NOTE: To implement scrolling correctly you must not use cursor::Move commands
    // TODO: Add an assert command to check the above note
    fn build(text: &str, width: usize) -> Result<Vec<String>, fmt::Error>;

    fn push(dest: &mut Vec<String>, src: impl Command) -> fmt::Result {
        let mut line = String::new();
        src.write_ansi(&mut line)?;
        dest.push(line);
        Ok(())
    }
}

macro_rules! body {
    ($width:expr, $($type:ident($text:expr)),*) => {
        vec![
            $(
                $type::build($text, ($width) as usize)?,
            )*
        ]
    };
}

pub struct Paragraph;

impl Component for Paragraph {
    fn build(text: &str, width: usize) -> Result<Vec<String>, fmt::Error> {
        const IDENT: usize = 4;
        let mut res = vec![String::new()];
        let text = " ".repeat(IDENT) + text;
        res.append(&mut wrap_text(&text, width)?);
        Ok(res)
    }
}

// TODO: Add the Title component
// TODO: Add the Lead component
// TODO: Add the List component
// TODO: Add the Boxed component

#[cfg(test)]
mod text_wrap_tests {
    use super::wrap_text;

    #[test]
    fn one_word() {
        let right = vec!["TEST"];
        assert_eq!(wrap_text("TEST", 10).unwrap(), right);
    }

    #[test]
    fn empty_line() {
        let right = vec![""];
        assert_eq!(wrap_text("", 10).unwrap(), right);
    }

    #[test]
    fn two_line() {
        let right = vec!["TEST", "TEST"];
        assert_eq!(wrap_text("TEST TEST", 5).unwrap(), right);
    }

    // TODO: Figure out how to warn if text couldn't wrap
    #[test]
    fn too_short() {
        let right = vec!["TEST", "TEST"];
        assert_eq!(wrap_text("TEST TEST", 3).unwrap(), right);
    }
}
