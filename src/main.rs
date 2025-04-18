mod backend;
mod frontend;

use backend::get_article;
use chrono::NaiveDateTime;
use crossterm::{
    QueueableCommand, cursor,
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers, poll, read},
    execute,
    terminal::{self, ClearType},
};
use frontend::{Components, Geometry, TextPad};
use std::{
    cell::RefCell,
    io::{self, Write, stdout},
    process,
    rc::Rc,
    thread,
    time::Duration,
};

pub enum Direction {
    Up,
    Down,
}

enum Controls {
    Quit,
    Resize((u16, u16)),
    MoveSelect(Direction),
    Scroll(Direction),
    Select,
}

pub enum View {
    Feed,
    Article,
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

// TODO: Deal with Feed/Article input
fn map_input(event: Event, view: View) -> Option<Controls> {
    match event {
        Event::Key(event) => {
            if event.kind == KeyEventKind::Press {
                // TODO: Add arrows for control
                match (event.code, view) {
                    (KeyCode::Char(c), view) => {
                        if event.modifiers.contains(KeyModifiers::CONTROL) && c == 'c' {
                            return Some(Controls::Quit);
                        }
                        match (c, view) {
                            ('k', View::Feed) => return Some(Controls::MoveSelect(Direction::Up)),
                            ('k', View::Article) => return Some(Controls::Scroll(Direction::Up)),
                            ('j', View::Feed) => {
                                return Some(Controls::MoveSelect(Direction::Down));
                            }
                            ('j', View::Article) => return Some(Controls::Scroll(Direction::Down)),
                            ('q', _) => return Some(Controls::Quit),
                            _ => return None,
                        }
                    }
                    (KeyCode::Enter, View::Feed) => return Some(Controls::Select),
                    (KeyCode::Backspace, _) => return Some(Controls::Quit),
                    _ => {}
                }
            }
        }
        Event::Resize(w, h) => return Some(Controls::Resize((w, h))),
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

fn run_article(article: Vec<Components>, geo: &Rc<RefCell<Geometry>>) -> io::Result<()> {
    let mut stdout = stdout();

    geo.borrow_mut().change_view(View::Article);
    let body = frontend::build_componenets(&article, geo.borrow().width as usize);
    let mut article_textpad = TextPad::new(body, geo)?;

    article_textpad.draw(&mut stdout)?;
    stdout.flush()?;

    let mut to_feed = false;
    while !to_feed {
        if poll(Duration::ZERO)? {
            match map_input(read()?, View::Article) {
                Some(Controls::Quit) => to_feed = true,
                Some(Controls::Resize(new_dimens)) => {
                    geo.borrow_mut().resize(new_dimens);
                    return run_article(article, geo);
                }
                Some(Controls::Scroll(dir)) => {
                    article_textpad.scroll_by(&mut stdout, dir, View::Article)?;
                    stdout.flush()?;
                }
                Some(Controls::Select) => {}
                Some(Controls::MoveSelect(_)) => {}
                None => {}
            }
        }
        thread::sleep(Duration::from_millis(16));
    }

    Ok(())
}

fn run_feed(mut feed: Feed, geo: &Rc<RefCell<Geometry>>) -> io::Result<()> {
    let mut stdout = stdout();

    let body = frontend::build_componenets(&feed.build(), geo.borrow().width as usize);
    feed.set_positions(&body);
    let mut feed_textpad = TextPad::new(body, geo)?;

    feed_textpad.draw(&mut stdout)?;
    feed.redraw_selected(&mut stdout, &mut feed_textpad, true)?;
    stdout.flush()?;

    let mut quit = false;
    while !quit {
        if poll(Duration::ZERO)? {
            match map_input(read()?, View::Feed) {
                Some(Controls::Quit) => quit = true,
                Some(Controls::MoveSelect(dir)) => {
                    feed.select(&mut stdout, &mut feed_textpad, dir)?;
                    stdout.flush()?;
                }
                Some(Controls::Resize(new_dimens)) => {
                    geo.borrow_mut().resize(new_dimens);
                    // TODO: Figure out a better way to have selected always displayed
                    feed.selected = 0;
                    return run_feed(feed, geo);
                }
                Some(Controls::Select) => {
                    let url = feed.get_selected_url();
                    // TODO: Figure out how to display errors
                    let article = get_article(url).unwrap();
                    run_article(article, geo)?;
                    geo.borrow_mut().change_view(View::Feed);
                    return run_feed(feed, geo);
                }
                Some(Controls::Scroll(_)) => {}
                None => {}
            }
        }
        thread::sleep(Duration::from_millis(16));
    }

    Ok(())
}

// TODO: Add a help command
fn main() {
    let feed = Feed::new().unwrap_or_else(|err| {
        eprintln!("Couldn't get feed: {err}");
        process::exit(1);
    });

    let _screen_state = ScreenState::enable().unwrap_or_else(|err| {
        eprintln!("Couldn't setup screen state: {err}");
        process::exit(1);
    });
    let dimens = terminal::size().unwrap_or_else(|err| {
        eprintln!("Couldn't get terminal dimentions: {err}");
        process::exit(1);
    });
    let geo = Geometry::new(dimens);
    let geo = Rc::new(RefCell::new(geo));
    run_feed(feed, &geo).unwrap_or_else(|err| {
        eprintln!("Display error: {err}");
        process::exit(1);
    });
}
