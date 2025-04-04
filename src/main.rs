#[macro_use]
mod frontend;

use frontend::*;

use std::{
    error::Error,
    fs,
    io::{Write, stdout},
    thread,
    time::Duration,
};

use crossterm::{
    QueueableCommand, cursor,
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers, poll, read},
    terminal::{self, ClearType},
};

enum Control {
    Quit,
    Resize(u16, u16),
    ScrollUp,
    ScrollDown,
}

fn handle_input(event: Event) -> Option<Control> {
    match event {
        Event::Key(event) => {
            if event.kind == KeyEventKind::Press {
                match event.code {
                    KeyCode::Char(c) => {
                        if event.modifiers.contains(KeyModifiers::CONTROL) && c == 'c' {
                            return Some(Control::Quit);
                        }
                        match c {
                            'k' => return Some(Control::ScrollDown),
                            'j' => return Some(Control::ScrollUp),
                            'q' => return Some(Control::Quit),
                            _ => {}
                        }
                    }
                    _ => {}
                }
            }
        }
        Event::Resize(nw, nh) => return Some(Control::Resize(nw, nh)),
        // TODO: Add mouse scrolling
        Event::Mouse(event) => match event.kind {
            /*
            MouseEventKind::ScrollUp => return Some(Control::ScrollDown),
            MouseEventKind::ScrollDown => return Some(Control::ScrollUp),
            */
            _ => {}
        },
        _ => {}
    }
    None
}

fn main() -> Result<(), Box<dyn Error>> {
    terminal::enable_raw_mode()?;
    let mut stdout = stdout();
    stdout
        .queue(event::EnableMouseCapture)?
        .queue(terminal::EnterAlternateScreen)?
        .queue(cursor::Hide)?
        .queue(terminal::Clear(ClearType::All))?;

    let (w, h) = terminal::size()?;

    let lorem_ipsum = fs::read_to_string("lorem_ipsum.txt")?;
    let mut lorem_ipsum = lorem_ipsum.lines();
    let title = lorem_ipsum.next().unwrap_or("NO TITLE");
    let paragraphs = lorem_ipsum.skip(1);
    let mut body = comp!(w / 2, Title(title));
    body.append(
        &mut paragraphs
            .map(|p| Paragraph::build(p, (w / 2) as usize).unwrap_or(vec!["NO PARAGRAPH".into()]))
            .collect(),
    );
    let mut article = Article::new(body, h, w)?;

    article.draw(&mut stdout)?;
    stdout.flush()?;

    let mut quit = false;
    while !quit {
        if poll(Duration::ZERO)? {
            match handle_input(read()?) {
                Some(Control::Quit) => quit = true,
                Some(Control::ScrollUp) => {
                    article.scroll_up(&mut stdout)?;
                    stdout.flush()?;
                }
                Some(Control::ScrollDown) => {
                    article.scroll_down(&mut stdout)?;
                    stdout.flush()?;
                }
                Some(Control::Resize(nw, nh)) => {
                    article.resize(nw, nh)?;
                    article.draw(&mut stdout)?;
                    stdout.flush()?;
                }
                None => {}
            }
        }
        thread::sleep(Duration::from_millis(16));
    }
    stdout
        .queue(event::DisableMouseCapture)?
        .queue(terminal::LeaveAlternateScreen)?
        .queue(terminal::Clear(ClearType::All))?
        .queue(cursor::Show)?
        .queue(cursor::MoveTo(0, 0))?;
    stdout.flush()?;
    terminal::disable_raw_mode()?;
    Ok(())
}
