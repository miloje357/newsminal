use crossterm::{cursor, style, QueueableCommand};
use std::io;

pub fn print_wrapping(qc: &mut impl QueueableCommand, text: &str, width: usize) -> io::Result<()> {
    let mut buf = String::new();
    for word in text.split(" ") {
        if buf.len() + word.len() + 1 > width {
            qc.queue(style::Print(&buf))?
                .queue(cursor::MoveDown(1))?
                .queue(cursor::MoveToColumn(1))?;
            buf.clear();
        }
        buf.push_str(word);
        buf.push(' ');
    }
    qc.queue(style::Print(&buf))?
        .queue(cursor::MoveDown(1))?
        .queue(cursor::MoveToColumn(1))?;
    Ok(())
}
