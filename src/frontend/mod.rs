// TODO: Make struct with built component string in it
mod controllers;

use crate::{ErrorWindow, FeedItem, View};
use crossterm::{
    QueueableCommand, cursor,
    style::{self, Color, ContentStyle, Stylize},
    terminal,
};
use std::{
    cell::RefCell,
    collections::VecDeque,
    io::{self, Write},
    rc::Rc,
};

#[derive(Debug, PartialEq)]
pub enum ComponentKind {
    Title(String),
    Subtitle(String),
    Lead(String),
    Paragraph(String),
    Boxed(Vec<String>),
}

#[derive(Debug, PartialEq)]
struct ComponentContent {
    lines: Vec<String>,
    posy: u16,
    style: Option<ContentStyle>,
}

#[derive(Debug, PartialEq)]
enum ComponentState {
    ToBuild,
    Built(ComponentContent),
}

#[derive(Debug, PartialEq)]
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

    fn build(&mut self, width: usize, posy: u16) {
        let lines = match &self.comp_type {
            ComponentKind::Title(text) => Title::build(&text, width),
            ComponentKind::Subtitle(text) => Subtitle::build(&text, width),
            ComponentKind::Lead(text) => Lead::build(&text, width),
            ComponentKind::Paragraph(text) => Paragraph::build(&text, width),
            ComponentKind::Boxed(text) => Boxed::build(&text.join("\n"), width),
        };
        self.content = ComponentState::Built(ComponentContent {
            lines,
            posy,
            style: None,
        });
    }

    fn content(&self, style: Option<ContentStyle>) -> Vec<String> {
        let (lines, comp_style) = match &self.content {
            ComponentState::ToBuild => panic!(
                "Couldn't get content because the component ({:#?}) wasn't built",
                self
            ),
            ComponentState::Built(content) => (content.lines.clone(), content.style),
        };
        let style = if style.is_some() { style } else { comp_style };
        if let Some(style) = style {
            lines
                .iter()
                .map(|line| style.apply(line).to_string())
                .collect()
        } else {
            lines
        }
    }

    fn get_posy(&self) -> u16 {
        match &self.content {
            ComponentState::ToBuild => panic!(
                "Couldn't get posy because the component ({:#?}) wasn't built",
                self
            ),
            ComponentState::Built(content) => content.posy,
        }
    }

    fn set_posy(&mut self, new_posy: u16) {
        match &mut self.content {
            ComponentState::ToBuild => panic!(
                "Couldn't set posy because the component ({:#?}) wasn't built",
                self
            ),
            ComponentState::Built(content) => {
                content.posy = new_posy;
            }
        }
    }

    fn is_built(&self) -> bool {
        self.content != ComponentState::ToBuild
    }

    fn height(&self) -> u16 {
        self.content(None).len() as u16
    }

    fn get_style(&self) -> Option<ContentStyle> {
        match &self.content {
            ComponentState::ToBuild => panic!(
                "Couldn't set posy because the component ({:#?}) wasn't built",
                self
            ),
            ComponentState::Built(content) => content.style,
        }
    }

    fn set_style(&mut self, new_style: ContentStyle) {
        match &mut self.content {
            ComponentState::ToBuild => panic!(
                "Couldn't set posy because the component ({:#?}) wasn't built",
                self
            ),
            ComponentState::Built(content) => {
                content.style = Some(new_style);
            }
        }
    }
}

impl From<ComponentKind> for Component {
    fn from(value: ComponentKind) -> Self {
        Self::new(value)
    }
}

#[derive(Debug)]
struct Components {
    items: VecDeque<Component>,
    width: usize,
}

impl Components {
    fn new(comps: Vec<ComponentKind>, width: usize) -> Self {
        let mut posy = 0;
        let comps: Vec<Component> = comps
            .into_iter()
            .map(|comp| {
                let mut comp: Component = comp.into();
                comp.build(width, posy);
                posy += comp.height();
                comp
            })
            .collect();
        Self {
            items: comps.into(),
            width,
        }
    }

    fn build(&mut self, new_width: usize) {
        let mut posy = 0;
        for comp in self.items.iter_mut() {
            if self.width == new_width && comp.is_built() {
                comp.set_posy(posy);
            } else {
                comp.build(new_width, posy);
            }
            posy += comp.height();
        }
        self.width = new_width;
    }

    fn to_lines(&self) -> Vec<String> {
        self.items
            .iter()
            .flat_map(|comp| comp.content(None))
            .collect()
    }

    fn push_front(&mut self, new_items: impl DoubleEndedIterator<Item = ComponentKind>) {
        for comp in new_items.rev() {
            self.items.push_front(comp.into());
        }
    }

    fn get(&self, index: usize) -> Option<&Component> {
        self.items.get(index)
    }

    fn get_mut(&mut self, index: usize) -> Option<&mut Component> {
        self.items.get_mut(index)
    }

    fn first(&self) -> &Component {
        self.items.front().expect("No loaded components")
    }

    fn last(&self) -> &Component {
        self.items.back().expect("No loaded components")
    }

    fn get_first_up_to(&self, last: u16, before: bool) -> Option<&Component> {
        let index = self.items.partition_point(|comp| comp.get_posy() < last);
        if before {
            if index == 0 {
                None
            } else {
                Some(&self.items[index - 1])
            }
        } else {
            self.items.get(index)
        }
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

// TODO: Add first_comp_index and last_comp_index
//       (consider if textpad should be able to scroll by component)
pub struct TextPad<'a> {
    components: Components,
    content: Vec<String>,
    first: u16,
    pub geo: &'a Rc<RefCell<Geometry>>,
}

impl<'a> TextPad<'a> {
    pub fn new(components: Vec<ComponentKind>, geo: &'a Rc<RefCell<Geometry>>) -> TextPad<'a> {
        let components = Components::new(components, geo.borrow().width as usize);
        Self {
            content: components.to_lines(),
            components,
            first: 0,
            geo,
        }
    }

    pub fn build_components(&mut self) {
        let width = self.geo.borrow().width as usize;
        self.components.build(width);
    }

    pub fn reset_content(&mut self) {
        self.content = self.components.to_lines();
    }

    pub fn build(&mut self) {
        self.build_components();
        self.reset_content();
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

    fn first_visible_comp(&self) -> &Component {
        self.components
            .get_first_up_to(self.first, true)
            .unwrap_or(self.components.first())
    }

    fn last_visible_comp(&self) -> &Component {
        let term_heigth = self.geo.borrow().term_height;
        self.components
            .get_first_up_to(self.first + term_heigth, false)
            .unwrap_or(self.components.last())
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

enum FeedItemColor {
    Read,
    New,
    NotNew,
    Selected,
    NotSelected,
}

impl FeedItemColor {
    fn to_style(&self, prev_style: Option<ContentStyle>) -> Option<ContentStyle> {
        match self {
            FeedItemColor::Read => Some(ContentStyle::new().dim()),
            FeedItemColor::New => Some(ContentStyle::new().blue()),
            FeedItemColor::NotNew => {
                if prev_style.and_then(|s| s.foreground_color) == Some(Color::Blue) {
                    Some(ContentStyle::new())
                } else {
                    None
                }
            }
            FeedItemColor::Selected => Some(ContentStyle::new().red()),
            FeedItemColor::NotSelected => {
                if prev_style.and_then(|s| s.foreground_color) == Some(Color::Red) {
                    Some(ContentStyle::new())
                } else {
                    None
                }
            }
        }
    }

    fn set_style(comp: &mut Component, color: Self) -> Option<()> {
        let prev_style = comp.get_style();
        if let Some(new_style) = color.to_style(prev_style) {
            comp.set_style(new_style);
            return Some(());
        }
        None
    }

    fn get_styled(comp: &Component, color: Self) -> Vec<String> {
        let prev_style = comp.get_style();
        let new_style = color.to_style(prev_style);
        comp.content(new_style)
    }
}

// TODO: Do a custom impl Buildable for FeedItem
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
        component.build(geo.width as usize, 0);
        let lines = component.content(Some(ContentStyle::new().red()));

        let starty = (geo.term_height - lines.len() as u16) / 2;
        qc.queue(terminal::Clear(terminal::ClearType::All))?
            .queue(cursor::MoveTo(geo.startx, starty))?;
        for line in lines {
            qc.queue(style::Print(line.red()))?
                .queue(cursor::MoveDown(1))?
                .queue(cursor::MoveToColumn(geo.startx))?;
        }
        Ok(())
    }

    pub fn resize(&self, term_dimens: (u16, u16)) {
        self.geo.borrow_mut().resize(term_dimens);
    }
}
