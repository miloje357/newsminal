mod backend;
mod frontend;
mod input;

use backend::{NewsSite, deserialize_parser, serialize_parser};
use chrono::{DateTime, Local};
use crossterm::{
    QueueableCommand, cursor,
    event::{self, Event},
    execute,
    terminal::{self, ClearType},
};
use frontend::{ComponentKind, Geometry, TextPad};
use input::*;
use log::LevelFilter;
use log4rs::append::file::FileAppender;
use log4rs::config::{Appender, Config, Root};
use log4rs::encode::pattern::PatternEncoder;
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use std::{
    cell::RefCell,
    collections::VecDeque,
    error::Error,
    io::{self, Write, stdout},
    panic, process,
    rc::Rc,
    thread,
    time::{Duration, Instant},
};

#[derive(Serialize, Deserialize)]
enum Body {
    Fetched { html: String, lead: String },
    ToFetch { url: String },
}

#[derive(Serialize, Deserialize)]
pub struct FeedItem {
    title: String,
    published: DateTime<Local>,
    body: Body,
    #[serde(
        serialize_with = "serialize_parser",
        deserialize_with = "deserialize_parser"
    )]
    parser: Rc<dyn NewsSite>,
}

pub struct Feed {
    time: Instant,
    items: VecDeque<FeedItem>,
    selected: usize,
    client: Client,
}

trait Runnable {
    fn run_every_minute(&mut self, _qc: impl QueueableCommand + Write) -> io::Result<()> {
        Ok(())
    }

    fn get_timer(&self) -> Option<Instant> {
        None
    }

    fn handle_input(&mut self, event: Event, qc: impl QueueableCommand + Write)
    -> io::Result<bool>;
    fn run(&mut self, mut qc: impl QueueableCommand + Write) -> io::Result<()> {
        let mut should_run = true;
        while should_run {
            if event::poll(Duration::ZERO)? {
                should_run = self.handle_input(event::read()?, &mut qc)?;
            }
            if let Some(timer) = self.get_timer() {
                if (Instant::now() - timer).as_secs() >= 60 {
                    self.run_every_minute(&mut qc)?;
                }
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
        content: Vec<ComponentKind>,
        geo: &'a Rc<RefCell<Geometry>>,
        mut qc: impl QueueableCommand + Write,
    ) -> io::Result<ArticleControler<'a>> {
        geo.borrow_mut().change_view(View::Article);
        let textpad = TextPad::new(content, geo);
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
                self.textpad.geo.borrow_mut().resize(new_dimens);
                self.textpad.build();
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
        let textpad = TextPad::new(content, geo);
        let feed_controler = Self {
            feed,
            textpad,
            input: InputBuffer::new(),
        };
        feed_controler.draw(&mut qc)?;
        qc.flush()?;
        Ok(feed_controler)
    }
}

impl Runnable for FeedControler<'_> {
    fn run_every_minute(&mut self, mut qc: impl QueueableCommand + Write) -> io::Result<()> {
        self.refresh(&mut qc)?;
        self.draw(&mut qc)?;
        qc.flush()?;
        Ok(())
    }

    fn get_timer(&self) -> Option<Instant> {
        Some(self.feed.time)
    }

    fn handle_input(
        &mut self,
        event: Event,
        mut qc: impl QueueableCommand + Write,
    ) -> io::Result<bool> {
        match self.input.map(event, View::Feed) {
            Some(Controls::Quit) => return Ok(false),
            Some(Controls::MoveSelect(dir)) => {
                self.move_select(&mut qc, dir, true)?;
                qc.flush()?;
            }
            Some(Controls::Resize(new_dimens)) => {
                self.textpad.geo.borrow_mut().resize(new_dimens);
                self.textpad.build();
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
                self.refresh(&mut qc)?;
                self.draw(&mut qc)?;
                qc.flush()?;
            }
            Some(Controls::Scroll(..)) => {}
            None => {}
        }
        Ok(true)
    }

    fn run(&mut self, mut qc: impl QueueableCommand + Write) -> io::Result<()> {
        let mut should_run = true;
        let mut timer = Instant::now();
        while should_run {
            if event::poll(Duration::ZERO)? {
                should_run = self.handle_input(event::read()?, &mut qc)?;
            }
            if let Some(new_timer) = self.get_timer() {
                timer = new_timer;
            }
            if (Instant::now() - timer).as_secs() >= 60 {
                self.run_every_minute(&mut qc)?;
                timer = Instant::now();
            }
            thread::sleep(Duration::from_millis(16));
        }
        Ok(())
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

fn init_logging() -> Result<(), Box<dyn Error>> {
    const PATTERN: &str = "{l} - {m}\n";
    const LOGGER_NAME: &str = "logfile";
    const MIN_LOG_LEVEL: LevelFilter = LevelFilter::Debug;

    let timestamp = Local::now().format("%Y-%m-%d_%H-%M-%S");
    let logfile = format!("logs/newsminal_{}.log", timestamp);

    let logfile = FileAppender::builder()
        .encoder(Box::new(PatternEncoder::new(PATTERN)))
        .build(logfile)?;

    let config = Config::builder()
        .appender(Appender::builder().build(LOGGER_NAME, Box::new(logfile)))
        .build(Root::builder().appender(LOGGER_NAME).build(MIN_LOG_LEVEL))?;

    log4rs::init_config(config)?;
    Ok(())
}

// TODO: Add a help command
fn main() -> io::Result<()> {
    init_logging().unwrap_or_else(|err| {
        eprintln!("Couldn't init logger: {err}");
        process::exit(1);
    });
    log::info!("Started logging");

    let feed = {
        #[cfg(feature = "testdata")]
        {
            use std::{fs::File, io::Read};

            let mut file = File::open("feed.json")?;
            let mut json = String::new();
            file.read_to_string(&mut json)?;
            Feed::from_json(json)?
        }
        #[cfg(not(feature = "testdata"))]
        {
            Feed::new().unwrap_or_else(|err| {
                eprintln!("Couldn't get feed: {err}");
                std::process::exit(1);
            })
        }
    };

    let default_hook = panic::take_hook();
    panic::set_hook(Box::new(move |info| {
        let _ = terminal::disable_raw_mode();
        let _ = execute!(stdout(), terminal::LeaveAlternateScreen);
        let _ = stdout().flush();
        default_hook(info)
    }));

    let _screen_state = ScreenState::enable()?;
    let dimens = terminal::size()?;
    let geo = Geometry::new(dimens);
    let geo = Rc::new(RefCell::new(geo));
    let mut stdout = stdout();
    let mut feed_controler = FeedControler::build(feed, &geo, &mut stdout)?;
    feed_controler.run(&mut stdout)
}
