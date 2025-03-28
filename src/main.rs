use std::{
    io::{self, stdout, Write},
    thread,
    time::Duration,
};

use crossterm::{
    cursor,
    event::{poll, read, Event, KeyCode, KeyEventKind},
    style,
    terminal::{self, ClearType},
    QueueableCommand,
};

fn main() -> io::Result<()> {
    terminal::enable_raw_mode()?;
    let mut stdout = stdout();
    stdout
        .queue(cursor::Hide)?
        .queue(terminal::Clear(ClearType::All))?;

    let (w, h) = terminal::size()?;
    let square_side = 10;
    for y in (h - square_side) / 2..(h + square_side) / 2 {
        for x in ((w - square_side * 2) / 2..(w + square_side * 2) / 2).step_by(2) {
            if (x / 2 + y) % 2 == 0 {
                stdout
                    .queue(cursor::MoveTo(x, y))?
                    .queue(style::Print("██"))?;
            }
        }
    }
    stdout.flush()?;

    let mut quit = false;
    while !quit {
        if poll(Duration::ZERO)? {
            if let Event::Key(event) = read()? {
                if event.kind == KeyEventKind::Press && event.code == KeyCode::Char('q') {
                    quit = true;
                }
            }
        }
        // stdout.flush()?;
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
