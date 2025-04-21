mod backend;
mod frontend;
mod input;

use backend::get_article;
use chrono::NaiveDateTime;
use crossterm::{
    QueueableCommand, cursor, event, execute,
    terminal::{self, ClearType},
};
use frontend::{Components, Geometry, TextPad};
use input::*;
use std::{
    cell::RefCell,
    io::{self, Write, stdout},
    process,
    rc::Rc,
    thread,
    time::Duration,
};

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
    let mut input = InputBuffer::new();
    while !to_feed {
        if event::poll(Duration::ZERO)? {
            match input.map(event::read()?, View::Article) {
                Some(Controls::Quit) => to_feed = true,
                Some(Controls::Resize(new_dimens)) => {
                    geo.borrow_mut().resize(new_dimens);
                    return run_article(article, geo);
                }
                Some(Controls::Scroll(dir)) => {
                    article_textpad.scroll(&mut stdout, dir, View::Article)?;
                    stdout.flush()?;
                }
                Some(Controls::GotoTop) => {
                    return run_article(article, geo);
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
    let mut input = InputBuffer::new();
    while !quit {
        if event::poll(Duration::ZERO)? {
            match input.map(event::read()?, View::Feed) {
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
                    feed.selected = 0;
                    return run_feed(feed, geo);
                }
                Some(Controls::GotoTop) => {
                    if feed.selected != 0 {
                        feed.selected = 0;
                        return run_feed(feed, geo);
                    }
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
