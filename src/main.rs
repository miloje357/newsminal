mod printer;

use std::{
    io::{self, stdout, Write},
    thread,
    time::Duration,
};

use crossterm::{
    cursor,
    event::{poll, read, Event, KeyCode, KeyEventKind},
    terminal::{self, ClearType},
    QueueableCommand,
};

fn main() -> io::Result<()> {
    terminal::enable_raw_mode()?;
    let mut stdout = stdout();
    stdout
        .queue(cursor::Hide)?
        .queue(terminal::Clear(ClearType::All))?;

    let lorem_ipsum = "Rem est et dolorum est enim corporis corporis. Voluptas excepturi cum veniam. Fuga ab tempore quis velit. Reiciendis dolorem occaecati accusamus animi. Impedit voluptatem tempore temporibus in voluptatem a eum nihil.";

    stdout.queue(cursor::MoveTo(1, 1))?;
    printer::print_wrapping(&mut stdout, lorem_ipsum, 30)?;
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
