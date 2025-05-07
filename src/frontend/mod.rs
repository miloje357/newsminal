mod controllers;

use crate::{ErrorWindow, FeedItem, View};
use crossterm::{
    QueueableCommand, cursor,
    style::{PrintStyledContent, Stylize},
    terminal,
};
use std::{
    cell::RefCell,
    collections::VecDeque,
    io::{self, Write},
    rc::Rc,
};

#[derive(Debug)]
pub enum ComponentKind {
    Title(String),
    Subtitle(String),
    Lead(String),
    Paragraph(String),
    Boxed(Vec<String>),
}

#[derive(Debug)]
enum ComponentState {
    ToBuild,
    Built(Vec<String>),
}

#[derive(Debug)]
pub struct Component {
    comp_type: ComponentKind,
    content: ComponentState,
}

impl Component {
    fn new(comp_type: ComponentKind) -> Self {
        Self {
            comp_type,
            content: ComponentState::ToBuild,
        }
    }

    fn build(&mut self, width: usize) {
        self.content = ComponentState::Built(match &self.comp_type {
            ComponentKind::Title(text) => Title::build(&text, width),
            ComponentKind::Subtitle(text) => Subtitle::build(&text, width),
            ComponentKind::Lead(text) => Lead::build(&text, width),
            ComponentKind::Paragraph(text) => Paragraph::build(&text, width),
            ComponentKind::Boxed(text) => Boxed::build(&text.join("\n"), width),
        });
    }

    fn content(&self) -> Option<Vec<String>> {
        match &self.content {
            ComponentState::ToBuild => None,
            ComponentState::Built(content) => Some(content.clone()),
        }
    }
}

impl From<ComponentKind> for Component {
    fn from(value: ComponentKind) -> Self {
        Self::new(value)
    }
}

// TODO: Consider adding a changed: bool field so that textpad only need to redraw when changed ==
//       true
pub struct Geometry {
    term_height: u16,
    term_width: u16,
    startx: u16,
    width: u16,
    max_width: u16,
}

// TODO: Implement configuration
impl Geometry {
    const FEED_WIDTH: u16 = 50;
    const ARTICLE_WIDTH: u16 = 70;

    pub fn new(term_dimens: (u16, u16)) -> Self {
        let (term_width, term_height) = term_dimens;
        let width = Self::FEED_WIDTH.min(term_width);
        let startx = (term_width - width) / 2;
        Self {
            term_height,
            term_width,
            startx,
            width,
            max_width: Self::FEED_WIDTH,
        }
    }

    pub fn change_view(&mut self, view: View) {
        self.max_width = match view {
            View::Feed => Self::FEED_WIDTH,
            View::Article => Self::ARTICLE_WIDTH,
            View::Error => Self::ARTICLE_WIDTH,
        };
        self.width = self.max_width.min(self.term_width);
        self.startx = (self.term_width - self.width) / 2;
    }

    // TODO: Consider returning term_width == width so that the textpad need to rebuild only when
    //       term_width == width
    pub fn resize(&mut self, term_dimens: (u16, u16)) {
        let (term_width, term_height) = term_dimens;
        self.term_width = term_width;
        self.term_height = term_height;
        self.width = (self.width.max(self.max_width)).min(self.term_width);
        self.startx = (self.term_width - self.width) / 2;
    }
}

pub struct TextPad<'a> {
    components: VecDeque<Component>,
    content: Vec<String>,
    first: u16,
    pub geo: &'a Rc<RefCell<Geometry>>,
}

impl<'a> TextPad<'a> {
    pub fn new(components: Vec<ComponentKind>, geo: &'a Rc<RefCell<Geometry>>) -> io::Result<Self> {
        let mut components: Vec<Component> =
            components.into_iter().map(|comp| comp.into()).collect();
        for comp in components.iter_mut() {
            comp.build(geo.borrow().width as usize);
        }
        Ok(Self {
            content: components
                .iter()
                .flat_map(|comp| comp.content().unwrap())
                .collect(),
            components: components.into(),
            first: 0,
            geo,
        })
    }

    fn comps_to_content(&mut self) {
        self.content = self
            .components
            .iter()
            .flat_map(|comp| comp.content().unwrap())
            .collect();
    }

    pub fn build(&mut self) {
        let width = self.geo.borrow().width as usize;
        for comp in self.components.iter_mut() {
            if comp.content().is_some() {
                continue;
            }
            comp.build(width);
        }
        self.comps_to_content();
    }

    pub fn resize(&mut self, term_dimens: (u16, u16)) {
        self.geo.borrow_mut().resize(term_dimens);
        let width = self.geo.borrow().width as usize;
        for comp in self.components.iter_mut() {
            comp.build(width);
        }
        self.comps_to_content();
    }

    pub fn draw(&self, mut qc: impl QueueableCommand + Write) -> io::Result<()> {
        let geo = self.geo.borrow();
        qc.queue(terminal::Clear(terminal::ClearType::All))?
            .queue(cursor::MoveTo(geo.startx, 0))?;
        for line in self
            .content
            .iter()
            .skip(self.first as usize)
            .take(geo.term_height as usize)
        {
            qc.write(line.as_bytes())?;
            qc.queue(cursor::MoveDown(1))?
                .queue(cursor::MoveToColumn(geo.startx))?;
        }
        Ok(())
    }

    fn scroll_by_lines(
        &mut self,
        mut qc: impl QueueableCommand + Write,
        lines: i16,
    ) -> io::Result<()> {
        let geo = self.geo.borrow();
        let mut draw_line = 0;
        let is_up = lines < 0;
        let mut lines = lines.abs() as u16;

        if is_up {
            if self.first < lines {
                lines = self.first;
            }
            self.first -= lines;

            qc.queue(terminal::ScrollDown(lines))?;
        } else {
            let last = self.first + geo.term_height;
            if (self.content.len() as u16) < geo.term_height {
                lines = 0
            } else if (last + lines) as usize >= self.content.len() {
                lines = self.content.len() as u16 - last;
            }
            self.first += lines;
            draw_line = geo.term_height - lines;

            qc.queue(terminal::ScrollUp(lines))?;
        }

        qc.queue(cursor::MoveTo(geo.startx, draw_line))?;
        for line in self
            .content
            .iter()
            .skip((self.first + draw_line) as usize)
            .take(lines as usize)
        {
            qc.write(line.as_bytes())?;
            qc.queue(cursor::MoveDown(1))?
                .queue(cursor::MoveToColumn(geo.startx))?;
        }
        Ok(())
    }
}

fn wrap_text(text: &str, width: usize) -> Vec<String> {
    let mut res = Vec::new();
    let mut buf = String::new();
    for word in text.split(" ") {
        if buf.len() + word.len() >= width {
            buf.pop();
            res.push(buf.clone());
            buf.clear();
        }
        buf.push_str(word);
        buf.push(' ');
    }
    res.push(buf.trim_end().to_string());
    res
}

pub trait Buildable {
    fn build(text: &str, width: usize) -> Vec<String>;
}

// BUG: Doesn't display utf-8 properly
// FIXME: Breaks make turn into \n which aren't accounted for
pub struct Paragraph;
impl Buildable for Paragraph {
    fn build(text: &str, width: usize) -> Vec<String> {
        const IDENT: usize = 4;
        let mut res = vec![String::new()];
        let text = " ".repeat(IDENT) + text.trim();
        res.append(&mut wrap_text(&text, width));
        res
    }
}

pub struct Title;
impl Buildable for Title {
    fn build(text: &str, width: usize) -> Vec<String> {
        let mut res = vec![String::new()];
        let wraped_text = wrap_text(text.trim(), width);
        let wraped_text = wraped_text.iter().map(|line| {
            let ident = (width - line.len()) / 2;
            let ident = " ".repeat(ident);
            // TODO: Style this better
            format!("{ident}{}{ident}", line.clone().on_dark_grey().bold())
        });
        res.extend(wraped_text);
        res
    }
}

pub struct Lead;
impl Buildable for Lead {
    fn build(text: &str, width: usize) -> Vec<String> {
        Paragraph::build(text, width)
            .iter()
            .map(|line| line.clone().bold().to_string())
            .collect()
    }
}

pub struct Subtitle;
impl Buildable for Subtitle {
    fn build(text: &str, width: usize) -> Vec<String> {
        let mut res = vec![String::new()];
        res.extend(
            Paragraph::build(text, width)
                .iter()
                .map(|line| line.clone().bold().to_string()),
        );
        res
    }
}

pub struct Boxed;
impl Buildable for Boxed {
    fn build(text: &str, width: usize) -> Vec<String> {
        let mut res = vec![" ".repeat(width).to_string()];
        res.push(format!(" ┌{}┐ ", "─".repeat(width - 4)));
        let mut text = text
            .split("\n")
            .flat_map(|p| Paragraph::build(p, width - 6))
            .map(|row| format!(" │ {row}{} │ ", " ".repeat(width - 6 - row.chars().count())));
        text.next();
        res.extend(text);
        res.push(format!(" └{}┘ ", "─".repeat(width - 4)));
        res
    }
}

impl FeedItem {
    pub fn build(&self) -> ComponentKind {
        let mut rows = vec![self.title.clone()];
        rows.push(self.published.to_string());
        ComponentKind::Boxed(rows)
    }
}

// TODO: Add the List component

impl ErrorWindow<'_> {
    pub fn draw(&self, mut qc: impl QueueableCommand + Write) -> io::Result<()> {
        let geo = self.geo.borrow();
        let component = vec![self.msg.clone()];
        let component = ComponentKind::Boxed(component);
        let mut component = Component::new(component);
        component.build(geo.width as usize);
        let lines = component.content().unwrap();

        let starty = (geo.term_height - lines.len() as u16) / 2;
        qc.queue(terminal::Clear(terminal::ClearType::All))?
            .queue(cursor::MoveTo(geo.startx, starty))?;
        for line in lines {
            qc.queue(PrintStyledContent(line.red()))?
                .queue(cursor::MoveDown(1))?
                .queue(cursor::MoveToColumn(geo.startx))?;
        }
        Ok(())
    }

    pub fn resize(&self, term_dimens: (u16, u16)) {
        self.geo.borrow_mut().resize(term_dimens);
    }
}
