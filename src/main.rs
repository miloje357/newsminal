mod backend;
mod frontend;
mod input;

use backend::NewsSite;
use chrono::NaiveDateTime;
use crossterm::{
    QueueableCommand, cursor,
    event::{self, Event},
    execute,
    terminal::{self, ClearType, disable_raw_mode},
};
use frontend::{Components, Geometry, TextPad};
use input::*;
use std::{
    cell::RefCell,
    collections::VecDeque,
    io::{self, Write, stdout},
    panic, process,
    rc::Rc,
    thread,
    time::Duration,
};

// TODO: Add read: bool field
pub struct FeedItem {
    url: String,
    title: String,
    published: NaiveDateTime,
    at: Option<usize>,
    parser: Rc<dyn NewsSite>,
}

pub struct Feed {
    time: NaiveDateTime,
    items: VecDeque<FeedItem>,
    selected: usize,
    page: usize,
}

trait Runnable {
    fn handle_input(&mut self, event: Event, qc: impl QueueableCommand + Write)
    -> io::Result<bool>;
    fn run(&mut self, mut qc: impl QueueableCommand + Write) -> io::Result<()> {
        let mut should_run = true;
        while should_run {
            if event::poll(Duration::ZERO)? {
                should_run = self.handle_input(event::read()?, &mut qc)?;
            }
            thread::sleep(Duration::from_millis(16));
        }
        Ok(())
    }
}

struct ErrorWindow<'a> {
    msg: String,
    geo: &'a Rc<RefCell<Geometry>>,
    input: InputBuffer,
}

impl<'a> ErrorWindow<'a> {
    pub fn build(msg: &str, geo: &'a Rc<RefCell<Geometry>>) -> io::Result<Self> {
        let mut stdout = stdout();
        let error_window = Self {
            msg: format!("(ERROR) {msg}"),
            geo,
            input: InputBuffer::new(),
        };
        error_window.draw(&mut stdout)?;
        stdout.flush()?;
        Ok(error_window)
    }
}

impl Runnable for ErrorWindow<'_> {
    fn handle_input(
        &mut self,
        event: Event,
        mut qc: impl QueueableCommand + Write,
    ) -> io::Result<bool> {
        match self.input.map(event, View::Error) {
            Some(Controls::Quit) => return Ok(false),
            Some(Controls::Resize(new_dimens)) => {
                self.resize(new_dimens);
                self.draw(&mut qc)?;
                qc.flush()?;
            }
            _ => {}
        }
        Ok(true)
    }
}

struct ArticleControler<'a> {
    textpad: TextPad<'a>,
    input: InputBuffer,
}

impl<'a> ArticleControler<'a> {
    pub fn build(
        content: Vec<Components>,
        geo: &'a Rc<RefCell<Geometry>>,
        mut qc: impl QueueableCommand + Write,
    ) -> io::Result<Self> {
        geo.borrow_mut().change_view(View::Article);
        let textpad = TextPad::new(content, geo)?;
        textpad.draw(&mut qc)?;
        qc.flush()?;
        Ok(Self {
            textpad,
            input: InputBuffer::new(),
        })
    }
}

impl Runnable for ArticleControler<'_> {
    fn handle_input(
        &mut self,
        event: Event,
        mut qc: impl QueueableCommand + Write,
    ) -> io::Result<bool> {
        match self.input.map(event, View::Article) {
            Some(Controls::Quit) => return Ok(false),
            Some(Controls::Resize(new_dimens)) => {
                self.textpad.resize(new_dimens);
                self.textpad.draw(&mut qc)?;
                qc.flush()?;
            }
            Some(Controls::Scroll(dir, lines)) => {
                self.scroll(&mut qc, dir, lines)?;
                qc.flush()?;
            }
            Some(Controls::GotoTop) => {
                self.goto_top();
                self.textpad.draw(&mut qc)?;
                qc.flush()?;
            }
            Some(Controls::Select) => {}
            Some(Controls::MoveSelect(_)) => {}
            Some(Controls::MouseSelect(..)) => {}
            Some(Controls::Refresh) => {}
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
    pub fn build(
        feed: Feed,
        geo: &'a Rc<RefCell<Geometry>>,
        mut qc: impl QueueableCommand + Write,
    ) -> io::Result<Self> {
        let content = feed.items.iter().map(|i| i.build()).collect::<Vec<_>>();
        let textpad = TextPad::new(content, geo)?;
        let mut feed_controler = Self {
            feed,
            textpad,
            input: InputBuffer::new(),
        };
        feed_controler.set_positions();
        feed_controler.textpad.draw(&mut qc)?;
        feed_controler.redraw_selected(&mut qc, true)?;
        qc.flush()?;
        Ok(feed_controler)
    }
}

impl Runnable for FeedControler<'_> {
    fn handle_input(
        &mut self,
        event: Event,
        mut qc: impl QueueableCommand + Write,
    ) -> io::Result<bool> {
        match self.input.map(event, View::Feed) {
            Some(Controls::Quit) => return Ok(false),
            Some(Controls::MoveSelect(dir)) => {
                let should_append = self.move_select(&mut qc, dir)?;
                if should_append {
                    self.append(&mut qc)?;
                    self.move_select(&mut qc, dir)?;
                }
                qc.flush()?;
            }
            Some(Controls::Resize(new_dimens)) => {
                self.textpad.resize(new_dimens);
                self.draw(&mut qc)?;
                qc.flush()?;
            }
            Some(Controls::Select) => {
                self.select(&mut qc)?;
                qc.flush()?;
            }
            Some(Controls::GotoTop) => {
                if self.feed.selected != 0 {
                    self.goto_top();
                    self.draw(&mut qc)?;
                    qc.flush()?;
                }
            }
            Some(Controls::MouseSelect(column, row)) => {
                let should_select = self.mouse_select(&mut qc, column, row)?;
                if should_select {
                    self.select(&mut qc)?;
                }
                qc.flush()?;
            }
            Some(Controls::Refresh) => {
                self.refresh(&mut qc, true)?;
                self.draw(&mut qc)?;
                qc.flush()?;
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
            cursor::Show,
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

    panic::set_hook(Box::new(|info| {
        let _ = disable_raw_mode();
        let _ = execute!(stdout(), terminal::LeaveAlternateScreen);
        let _ = stdout().flush();
        eprintln!("Panic occurred: {}", info);
    }));

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
    let mut stdout = stdout();
    let mut feed_controler = FeedControler::build(feed, &geo, &mut stdout).unwrap_or_else(|err| {
        eprintln!("Couldn't init the FeedControler: {err}");
        process::exit(1);
    });
    feed_controler.run(&mut stdout).unwrap_or_else(|err| {
        eprintln!("Display error: {err}");
        process::exit(1);
    });
}
