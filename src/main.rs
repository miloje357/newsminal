mod backend;
mod frontend;

use chrono::NaiveDateTime;
use crossterm::{
    QueueableCommand, cursor,
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers, poll, read},
    execute,
    terminal::{self, ClearType},
};
use frontend::{Components, TextPad};
use std::{
    io::{self, Write, stdout},
    process, thread,
    time::Duration,
};

enum ScrollType {
    UpByLine,
    UpByFeedItem,
    DownByLine,
    DownByFeedItem,
}

enum Control {
    Quit,
    Resize(u16, u16),
    Scroll(ScrollType),
}

struct FeedItem {
    url: Option<String>,
    title: String,
    published: Option<NaiveDateTime>,
}

pub struct Feed {
    time: NaiveDateTime,
    items: Vec<FeedItem>,
}

fn map_input(event: Event) -> Option<Control> {
    match event {
        Event::Key(event) => {
            if event.kind == KeyEventKind::Press {
                // TODO: Add arrows for control
                match event.code {
                    KeyCode::Char(c) => {
                        if event.modifiers.contains(KeyModifiers::CONTROL) && c == 'c' {
                            return Some(Control::Quit);
                        }
                        match c {
                            'k' => return Some(Control::Scroll(ScrollType::UpByFeedItem)),
                            'j' => return Some(Control::Scroll(ScrollType::DownByFeedItem)),
                            'q' => return Some(Control::Quit),
                            _ => {}
                        }
                    }
                    _ => {}
                }
            }
        }
        // BUG: Doesn't work
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

struct ScreenState;

impl ScreenState {
    fn enable() -> io::Result<Self> {
        terminal::enable_raw_mode()?;
        stdout()
            .queue(event::EnableMouseCapture)?
            .queue(terminal::EnterAlternateScreen)?
            .queue(cursor::Hide)?
            .queue(terminal::Clear(ClearType::All))?;
        Ok(Self)
    }
}

impl Drop for ScreenState {
    fn drop(&mut self) {
        let _ = execute!(
            stdout(),
            event::DisableMouseCapture,
            terminal::LeaveAlternateScreen,
            terminal::Clear(ClearType::All),
            cursor::Show,
            cursor::MoveTo(0, 0)
        )
        .unwrap_or_else(|err| eprintln!("Display error: {err}"));
        terminal::disable_raw_mode()
            .unwrap_or_else(|err| eprintln!("Couldn't leave raw mode {err}"));
    }
}

fn run(feed: Vec<Components>) -> io::Result<()> {
    let _screen_state = ScreenState::enable()?;
    let mut stdout = stdout();
    let (w, h) = terminal::size()?;

    // TODO: Add article geometry configuration
    let body = frontend::build_componenets(feed, (w / 2).into());
    let mut article = TextPad::new(body, h, w)?;

    article.draw(&mut stdout)?;
    stdout.flush()?;

    let mut quit = false;
    while !quit {
        if poll(Duration::ZERO)? {
            match map_input(read()?) {
                Some(Control::Quit) => quit = true,
                Some(Control::Scroll(st)) => {
                    article.scroll_by(&mut stdout, st)?;
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

    Ok(())
}

// TODO: Add the feed
// TODO: Add a help command
fn main() {
    let feed = Feed::new()
        .unwrap_or_else(|err| {
            eprintln!("Couldn't get feed: {err}");
            process::exit(1);
        })
        .build();

    run(feed).unwrap_or_else(|err| {
        eprintln!("Display error: {err}");
        process::exit(1);
    });
}
