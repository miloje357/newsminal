mod backend;
mod frontend;
mod input;

use backend::get_article;
use chrono::NaiveDateTime;
use crossterm::{
    QueueableCommand, cursor,
    event::{self, Event},
    execute,
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

trait Runnable {
    fn handle_input(&mut self, event: Event) -> io::Result<bool>;
    fn run(&mut self) -> io::Result<()> {
        let mut should_run = true;
        while should_run {
            if event::poll(Duration::ZERO)? {
                should_run = self.handle_input(event::read()?)?;
            }
            thread::sleep(Duration::from_millis(16));
        }
        Ok(())
    }
}

struct ArticleControler<'a> {
    textpad: TextPad<'a>,
    input: InputBuffer,
}

impl<'a> ArticleControler<'a> {
    pub fn build(content: Vec<Components>, geo: &'a Rc<RefCell<Geometry>>) -> io::Result<Self> {
        let mut stdout = stdout();
        geo.borrow_mut().change_view(View::Article);
        let textpad = TextPad::new(content, geo)?;
        textpad.draw(&mut stdout)?;
        stdout.flush()?;
        Ok(Self {
            textpad,
            input: InputBuffer::new(),
        })
    }
}

impl Runnable for ArticleControler<'_> {
    fn handle_input(&mut self, event: Event) -> io::Result<bool> {
        let mut stdout = stdout();
        match self.input.map(event, View::Article) {
            Some(Controls::Quit) => return Ok(false),
            Some(Controls::Resize(new_dimens)) => {
                self.textpad.resize(new_dimens);
                self.textpad.draw(&mut stdout)?;
                stdout.flush()?;
            }
            Some(Controls::Scroll(dir, lines)) => {
                self.scroll(&mut stdout, dir, lines)?;
                stdout.flush()?;
            }
            Some(Controls::GotoTop) => {
                self.goto_top();
                self.textpad.draw(&mut stdout)?;
                stdout.flush()?;
            }
            Some(Controls::Select) => {}
            Some(Controls::MoveSelect(_)) => {}
            None => {}
        }
        Ok(true)
    }
}

struct FeedControler<'a> {
    feed: Feed,
    textpad: TextPad<'a>,
    input: InputBuffer,
}

impl<'a> FeedControler<'a> {
    pub fn build(feed: Feed, geo: &'a Rc<RefCell<Geometry>>) -> io::Result<Self> {
        let content = feed.items.iter().map(|i| i.build()).collect::<Vec<_>>();
        let textpad = TextPad::new(content, geo)?;
        let mut feed_controler = Self {
            feed,
            textpad,
            input: InputBuffer::new(),
        };
        feed_controler.set_positions();
        let mut stdout = stdout();
        feed_controler.textpad.draw(&mut stdout)?;
        feed_controler.redraw_selected(&mut stdout, true)?;
        stdout.flush()?;
        Ok(feed_controler)
    }
}

impl Runnable for FeedControler<'_> {
    fn handle_input(&mut self, event: Event) -> io::Result<bool> {
        let mut stdout = stdout();
        match self.input.map(event, View::Feed) {
            Some(Controls::Quit) => return Ok(false),
            Some(Controls::MoveSelect(dir)) => {
                self.select(&mut stdout, dir)?;
                stdout.flush()?;
            }
            Some(Controls::Resize(new_dimens)) => {
                self.textpad.resize(new_dimens);
                self.draw(&mut stdout)?;
                stdout.flush()?;
            }
            Some(Controls::Select) => {
                let url = self.feed.get_selected_url();
                // TODO: Figure out how to display errors
                let article = get_article(url).unwrap();
                ArticleControler::build(article, self.textpad.geo)?.run()?;
                self.change_view();
                self.draw(&mut stdout)?;
                stdout.flush()?;
            }
            Some(Controls::GotoTop) => {
                if self.feed.selected != 0 {
                    self.goto_top();
                    self.draw(&mut stdout)?;
                    stdout.flush()?;
                }
            }
            Some(Controls::Scroll(..)) => {}
            None => {}
        }
        Ok(true)
    }
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
    let mut feed_controler = FeedControler::build(feed, &geo).unwrap_or_else(|err| {
        eprintln!("Couldn't init the FeedControler: {err}");
        process::exit(1);
    });
    feed_controler.run().unwrap_or_else(|err| {
        eprintln!("Display error: {err}");
        process::exit(1);
    });
}
