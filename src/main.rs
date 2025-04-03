#[macro_use]
mod frontend;

use frontend::*;

use std::{
    error::Error,
    io::{stdout, Write},
    thread,
    time::Duration,
};

use crossterm::{
    cursor,
    event::{poll, read, Event, KeyCode, KeyEventKind},
    terminal::{self, ClearType},
    QueueableCommand,
};

fn main() -> Result<(), Box<dyn Error>> {
    terminal::enable_raw_mode()?;
    let mut stdout = stdout();
    stdout
        .queue(cursor::Hide)?
        .queue(terminal::Clear(ClearType::All))?;

    let (w, h) = terminal::size()?;

    let lorem_ipsum = "Rem est et dolorum est enim corporis corporis. Voluptas excepturi cum veniam. Fuga ab tempore quis velit. Reiciendis dolorem occaecati accusamus animi. Impedit voluptatem tempore temporibus in voluptatem a eum nihil.";

    let article = Article::new(
        body!(
            w / 2,
            Paragraph(lorem_ipsum),
            Paragraph(lorem_ipsum),
            Paragraph(lorem_ipsum)
        ),
        h as usize,
    )?;

    article.draw(&mut stdout)?;
    stdout.flush()?;

    let mut quit = false;
    while !quit {
        if poll(Duration::ZERO)? {
            // TODO: Add keyboard scrolling
            // TODO: Add mouse scrolling
            if let Event::Key(event) = read()? {
                if event.kind == KeyEventKind::Press && event.code == KeyCode::Char('q') {
                    quit = true;
                }
            }
        }
        thread::sleep(Duration::from_millis(16));
    }
    stdout
        .queue(terminal::Clear(ClearType::All))?
        .queue(cursor::Show)?
        .queue(cursor::MoveTo(0, 0))?;
    stdout.flush()?;
    terminal::disable_raw_mode()?;
    Ok(())
}
