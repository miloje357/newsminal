use std::io::{self, Write};

use crossterm::{
    QueueableCommand, cursor,
    style::{self, Stylize},
};

use crate::{
    ArticleControler, FeedControler,
    input::{Direction, View},
};

use super::build_componenets;

impl ArticleControler<'_> {
    pub fn scroll(
        &mut self,
        mut qc: impl QueueableCommand + Write,
        dir: Direction,
        lines: u16,
    ) -> io::Result<()> {
        match dir {
            Direction::Up => self.textpad.scroll_by_lines(&mut qc, -(lines as i16))?,
            Direction::Down => self.textpad.scroll_by_lines(&mut qc, lines as i16)?,
        }
        Ok(())
    }

    pub fn goto_top(&mut self) {
        self.textpad.first = 0;
    }
}

impl FeedControler<'_> {
    fn scroll(&mut self, mut qc: impl QueueableCommand + Write, dir: Direction) -> io::Result<()> {
        match dir {
            Direction::Up => {
                let lines = self
                    .textpad
                    .content
                    .iter()
                    .take(self.textpad.first as usize)
                    .rev()
                    .take_while(|line| !line.chars().all(|c| c.is_whitespace()))
                    .count() as i16
                    + 1;
                self.textpad.scroll_by_lines(&mut qc, -lines)?;
            }
            Direction::Down => {
                let term_height = self.textpad.geo.borrow().term_height;
                let lines = self
                    .textpad
                    .content
                    .iter()
                    .skip((self.textpad.first + term_height) as usize + 1)
                    .take_while(|line| !line.chars().all(|c| c.is_whitespace()))
                    .count() as i16
                    + 1;
                self.textpad.scroll_by_lines(&mut qc, lines)?;
            }
        }
        Ok(())
    }
    pub fn redraw_selected(
        &self,
        mut qc: impl QueueableCommand + Write,
        is_selected: bool,
    ) -> io::Result<()> {
        let startx = self.textpad.geo.borrow().startx;
        let first_row = self.feed.items[self.feed.selected].at.unwrap();
        let last_row = {
            if self.feed.selected + 1 >= self.feed.items.len() {
                self.textpad.content.len()
            } else {
                self.feed.items[self.feed.selected + 1].at.unwrap()
            }
        };
        let item_content = &self.textpad.content[first_row..last_row];
        qc.queue(cursor::MoveTo(
            startx,
            first_row as u16 - self.textpad.first,
        ))?;
        for line in item_content {
            if is_selected {
                qc.queue(style::PrintStyledContent(line.clone().red()))?;
            } else {
                qc.queue(style::Print(line))?;
            }
            qc.queue(cursor::MoveDown(1))?
                .queue(cursor::MoveToColumn(startx))?;
        }
        Ok(())
    }

    pub fn select(
        &mut self,
        mut qc: impl QueueableCommand + Write,
        dir: Direction,
    ) -> io::Result<()> {
        let term_height = self.textpad.geo.borrow().term_height;
        self.redraw_selected(&mut qc, false)?;
        match dir {
            Direction::Up => {
                if self.feed.selected > 0 {
                    self.feed.selected -= 1;
                    if self.feed.selected > 0 {
                        let next_row = self.feed.items[self.feed.selected - 1].at.unwrap() as u16;
                        if next_row < self.textpad.first {
                            self.scroll(&mut qc, dir)?;
                        }
                    }
                }
            }
            Direction::Down => {
                if self.feed.selected < self.feed.items.len() - 1 {
                    self.feed.selected += 1;
                    let next_row = if self.feed.selected < self.feed.items.len() - 2 {
                        self.feed.items[self.feed.selected + 2].at.unwrap() as u16
                    } else if self.feed.selected < self.feed.items.len() - 1 {
                        self.feed.items[self.feed.selected + 1].at.unwrap() as u16
                    } else {
                        self.textpad.first
                    };
                    if next_row - self.textpad.first >= term_height {
                        self.scroll(&mut qc, dir)?;
                    }
                }
            }
        };
        self.redraw_selected(&mut qc, true)?;
        Ok(())
    }

    pub fn set_positions(&mut self) {
        let mut items = self.feed.items.iter_mut();
        for (i, line) in self.textpad.content.iter().enumerate() {
            if line.chars().all(|c| c.is_whitespace()) {
                if let Some(item) = items.next() {
                    item.at = Some(i);
                }
            }
        }
    }

    pub fn change_view(&mut self) {
        self.textpad.geo.borrow_mut().change_view(View::Feed);
        self.textpad.content = build_componenets(
            &self.textpad.components,
            self.textpad.geo.borrow().width as usize,
        );
    }

    pub fn draw(&self, mut qc: impl QueueableCommand + Write) -> io::Result<()> {
        self.textpad.draw(&mut qc)?;
        self.redraw_selected(&mut qc, true)?;
        Ok(())
    }

    pub fn goto_top(&mut self) {
        self.feed.selected = 0;
        self.textpad.first = 0;
    }
}
