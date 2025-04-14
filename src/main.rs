#[allow(dead_code)]
mod backend;
mod frontend;

use chrono::NaiveDateTime;
use crossterm::{
    QueueableCommand, cursor,
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers, poll, read},
    execute,
    terminal::{self, ClearType},
};
use frontend::TextPad;
use std::{
    io::{self, Write, stdout},
    process, thread,
    time::Duration,
};

pub enum ScrollType {
    UpByLine,
    UpByFeedItem,
    DownByLine,
    DownByFeedItem,
}

enum ArticleControls {
    Quit,
    Resize(u16, u16),
    Scroll(ScrollType),
}

enum FeedControls {
    Quit,
    Resize(u16, u16),
    Select(ScrollType),
}

struct FeedItem {
    url: Option<String>,
    title: String,
    published: Option<NaiveDateTime>,
    at: Option<usize>,
}

pub struct Feed {
    time: NaiveDateTime,
    items: Vec<FeedItem>,
    selected: usize,
}

fn map_input(event: Event) -> Option<FeedControls> {
    match event {
        Event::Key(event) => {
            if event.kind == KeyEventKind::Press {
                // TODO: Add arrows for control
                match event.code {
                    KeyCode::Char(c) => {
                        if event.modifiers.contains(KeyModifiers::CONTROL) && c == 'c' {
                            return Some(FeedControls::Quit);
                        }
                        match c {
                            'k' => return Some(FeedControls::Select(ScrollType::UpByFeedItem)),
                            'j' => return Some(FeedControls::Select(ScrollType::DownByFeedItem)),
                            'q' => return Some(FeedControls::Quit),
                            _ => {}
                        }
                    }
                    _ => {}
                }
            }
        }
        // BUG: Doesn't work
        Event::Resize(nw, nh) => return Some(FeedControls::Resize(nw, nh)),
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
        execute!(
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

fn run(mut feed: Feed) -> io::Result<()> {
    let _screen_state = ScreenState::enable()?;
    let mut stdout = stdout();
    let (w, h) = terminal::size()?;

    // TODO: Add article geometry configuration
    let body = frontend::build_componenets(feed.build(), (w / 2).into());
    feed.set_positions(&body);
    let mut feed_textpad = TextPad::new(body, h, w)?;

    feed_textpad.draw(&mut stdout)?;
    feed.redraw_selected(&mut stdout, &mut feed_textpad, true)?;
    stdout.flush()?;

    let mut quit = false;
    while !quit {
        if poll(Duration::ZERO)? {
            match map_input(read()?) {
                Some(FeedControls::Quit) => quit = true,
                Some(FeedControls::Select(st)) => {
                    feed.select(&mut stdout, &mut feed_textpad, st)?;
                    stdout.flush()?;
                }
                Some(FeedControls::Resize(nw, nh)) => {
                    feed_textpad.resize(nw, nh)?;
                    feed_textpad.draw(&mut stdout)?;
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
    let feed = Feed::new().unwrap_or_else(|err| {
        eprintln!("Couldn't get feed: {err}");
        process::exit(1);
    });

    run(feed).unwrap_or_else(|err| {
        eprintln!("Display error: {err}");
        process::exit(1);
    });
}
