use std::{
    io::{self, Write},
    time::Duration,
};

use crossterm::{QueueableCommand, cursor, event, style, terminal};

use crate::{
    ArticleControler, ErrorWindow, FeedControler, Runnable,
    input::{Direction, View},
};

use super::FeedItemColor;

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
                let lines: i16 =
                    self.textpad.first as i16 - self.textpad.first_visible_comp().get_posy() as i16;
                self.textpad.scroll_by_lines(&mut qc, -lines)?;
            }
            Direction::Down => {
                let term_height = self.textpad.geo.borrow().term_height;
                let last = self.textpad.last_visible_comp();
                let mut lines =
                    (last.get_posy() + last.height()) - (self.textpad.first + term_height);
                if lines == 0 {
                    lines = last.height();
                }
                self.textpad.scroll_by_lines(&mut qc, lines as i16)?;
            }
        }
        Ok(())
    }

    fn redraw_selected(
        &self,
        mut qc: impl QueueableCommand + Write,
        color: FeedItemColor,
    ) -> io::Result<()> {
        let startx = self.textpad.geo.borrow().startx;
        let comp = self
            .textpad
            .components
            .get(self.feed.selected)
            .expect("Selected feeditem is not loaded");
        qc.queue(cursor::MoveTo(startx, comp.get_posy() - self.textpad.first))?;
        for line in FeedItemColor::get_styled(comp, color) {
            qc.queue(style::Print(line))?
                .queue(cursor::MoveDown(1))?
                .queue(cursor::MoveToColumn(startx))?;
        }
        Ok(())
    }

    fn rebuild_selected(&mut self, color: FeedItemColor) {
        let selected = self
            .textpad
            .components
            .get_mut(self.feed.selected)
            .expect("Selected not loaded");
        let drawn_something = FeedItemColor::set_style(selected, color);
        if drawn_something.is_some() {
            // TODO: Consider writing a reset_content that appends to the textpad.content instead
            //       of reseting it all
            self.textpad.reset_content();
        }
    }

    fn num_to_scroll_down(&self) -> usize {
        let last_visible = self.textpad.last_visible_comp();
        if Some(last_visible) == self.textpad.components.get(self.feed.selected) {
            2
        } else if Some(last_visible) == self.textpad.components.get(self.feed.selected + 1) {
            1
        } else {
            0
        }
    }

    fn num_to_scroll_up(&self) -> usize {
        let first_visible = self.textpad.first_visible_comp();
        if Some(first_visible) == self.textpad.components.get(self.feed.selected) {
            2
        } else if Some(first_visible) == self.textpad.components.get(self.feed.selected - 1) {
            1
        } else {
            0
        }
    }

    pub fn move_select(
        &mut self,
        mut qc: impl QueueableCommand + Write,
        dir: Direction,
        should_remove_new: bool,
    ) -> io::Result<()> {
        self.redraw_selected(&mut qc, FeedItemColor::NotSelected)?;
        match dir {
            Direction::Up => {
                if self.feed.selected > 0 {
                    self.feed.selected -= 1;
                }
                if self.num_to_scroll_up() != 0 {
                    self.scroll(&mut qc, dir)?;
                }
            }
            Direction::Down => {
                if self.feed.selected < self.feed.items.len() - 1 {
                    self.feed.selected += 1;
                }
                if self.num_to_scroll_down() != 0 {
                    self.scroll(&mut qc, dir)?;
                }
            }
        };
        if should_remove_new {
            self.rebuild_selected(FeedItemColor::NotNew);
        }
        self.redraw_selected(&mut qc, FeedItemColor::Selected)?;
        Ok(())
    }

    pub fn draw(&self, mut qc: impl QueueableCommand + Write) -> io::Result<()> {
        self.textpad.draw(&mut qc)?;
        self.redraw_selected(&mut qc, FeedItemColor::Selected)?;
        Ok(())
    }

    pub fn goto_top(&mut self) {
        self.feed.selected = 0;
        self.textpad.first = 0;
    }

    pub fn mouse_select(
        &mut self,
        mut qc: impl QueueableCommand + Write,
        x: u16,
        y: u16,
    ) -> io::Result<bool> {
        let geo = self.textpad.geo.borrow();
        let x = x as i16 - geo.startx as i16;
        if x < 0 || x > geo.width as i16 {
            return Ok(false);
        }
        self.redraw_selected(&mut qc, FeedItemColor::NotSelected)?;
        let last_selected = self.feed.selected;
        self.feed.selected = self
            .textpad
            .components
            .items
            .partition_point(|i| y + self.textpad.first >= i.get_posy() + i.height());
        if self.feed.selected > last_selected {
            for _ in 0..self.num_to_scroll_down() {
                self.scroll(&mut qc, Direction::Down)?;
            }
        } else if self.feed.selected < last_selected {
            for _ in 0..self.num_to_scroll_up() {
                self.scroll(&mut qc, Direction::Up)?;
            }
        }
        self.rebuild_selected(FeedItemColor::NotNew);
        self.redraw_selected(&mut qc, FeedItemColor::Selected)?;
        Ok(self.feed.selected == last_selected)
    }

    fn draw_refreshing(
        &self,
        mut qc: impl QueueableCommand + Write,
        heigth: u16,
    ) -> io::Result<()> {
        const TEXT: &str = "REFRESHING...";
        let geo = self.textpad.geo.borrow();
        let x = geo.startx + (geo.width - TEXT.len() as u16) / 2;
        qc.queue(cursor::MoveTo(x, heigth / 2 + 1))?
            .queue(style::Print(TEXT))?;
        Ok(())
    }

    pub fn refresh(&mut self, mut qc: impl QueueableCommand + Write) -> io::Result<()> {
        const HEIGHT: u16 = 5;
        qc.queue(terminal::ScrollDown(HEIGHT))?;
        self.draw_refreshing(&mut qc, HEIGHT)?;
        qc.flush()?;
        let num_new = self.feed.refresh();
        qc.queue(terminal::ScrollUp(HEIGHT))?;
        // Clear stdin
        while event::poll(Duration::ZERO)? {
            let _ = event::read();
        }

        if num_new == Some(0) {
            return Ok(());
        }
        if let Some(num_new) = num_new {
            let new_comps = self.feed.items.iter().take(num_new).map(|i| i.build());
            self.textpad.components.push_front(new_comps);
            self.textpad.build_components();
            for comp in self.textpad.components.items.iter_mut().take(num_new) {
                FeedItemColor::set_style(comp, FeedItemColor::New);
            }
            self.textpad.reset_content();
            for _ in 0..num_new {
                self.move_select(&mut qc, Direction::Down, false)?;
            }
        }
        // TODO: Consider writing a more optimized draw for this situtation
        Ok(())
    }

    pub fn select(&mut self, mut qc: impl QueueableCommand + Write) -> io::Result<()> {
        // TODO: Add a loading page
        self.input.clear();
        match self.feed.selected().get_article() {
            Ok(article) => {
                ArticleControler::build(article, self.textpad.geo, &mut qc)?.run(&mut qc)?
            }
            Err(err) => ErrorWindow::build(
                &format!("Couldn't get article content: {err}"),
                self.textpad.geo,
            )?
            .run(&mut qc)?,
        }
        self.textpad.geo.borrow_mut().change_view(View::Feed);
        self.rebuild_selected(FeedItemColor::Read);
        // FIXME: Add self.textpad.resize();
        self.draw(&mut qc)?;
        Ok(())
    }
}
