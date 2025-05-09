use std::{
    io::{self, Write},
    time::Duration,
};

use crossterm::{
    QueueableCommand, cursor, event,
    style::{self, Stylize},
    terminal,
};

use crate::{
    ArticleControler, ErrorWindow, FeedControler, Runnable,
    input::{Direction, View},
};

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
                    self.textpad.first as i16 - self.textpad.first_visible_comp().posy() as i16;
                self.textpad.scroll_by_lines(&mut qc, -lines)?;
            }
            Direction::Down => {
                let term_height = self.textpad.geo.borrow().term_height;
                let last = self.textpad.last_visible_comp();
                let mut lines: i16 =
                    last.posy() as i16 - (self.textpad.first as i16 + term_height as i16);
                if lines == 0 {
                    lines = last.content().len() as i16
                }
                self.textpad.scroll_by_lines(&mut qc, lines)?;
            }
        }
        Ok(())
    }

    // FIXME: Do with styled Components
    pub fn redraw_selected(
        &self,
        mut qc: impl QueueableCommand + Write,
        is_selected: bool,
    ) -> io::Result<()> {
        let startx = self.textpad.geo.borrow().startx;
        let comp = &self.textpad.components.items[self.feed.selected];
        qc.queue(cursor::MoveTo(startx, comp.posy() - self.textpad.first))?;
        for line in comp.content() {
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

    pub fn move_select(
        &mut self,
        mut qc: impl QueueableCommand + Write,
        dir: Direction,
    ) -> io::Result<()> {
        self.redraw_selected(&mut qc, false)?;
        match dir {
            Direction::Up => {
                if self.feed.selected > 0 {
                    self.feed.selected -= 1;
                }
                if self.feed.selected > 0
                    && self.textpad.first_visible_comp()
                        == &self.textpad.components.items[self.feed.selected - 1]
                {
                    self.scroll(&mut qc, dir)?;
                }
            }
            Direction::Down => {
                if self.feed.selected < self.feed.items.len() - 1 {
                    self.feed.selected += 1;
                }
                // BUG: Scrolls weirdly when end is reached
                let last_visible = self.textpad.last_visible_comp();
                if (self.feed.selected < self.feed.items.len() - 2
                    && last_visible == &self.textpad.components.items[self.feed.selected + 2])
                    || (self.feed.selected < self.feed.items.len() - 1
                        && last_visible == &self.textpad.components.items[self.feed.selected + 1])
                {
                    self.scroll(&mut qc, dir)?;
                }
            }
        };
        self.redraw_selected(&mut qc, true)?;
        Ok(())
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
        self.redraw_selected(&mut qc, false)?;
        // TODO: Optimize
        let last_selected = self.feed.selected;
        if let Some(selected) = self
            .textpad
            .components
            .items
            .iter()
            .position(|i| y + self.textpad.first < i.posy())
        {
            self.feed.selected = selected - 1;
        }
        // BUG: Doesn't work when you select a top out-of-bounds feeditem
        if let Some(after_selected) = self
            .textpad
            .components
            .items
            .iter()
            .skip(self.feed.selected + 1)
            .next()
        {
            let last_at = after_selected.posy();
            let last_line = self.textpad.first + geo.term_height;
            if last_at > last_line {
                self.scroll(&mut qc, Direction::Down)?;
                self.scroll(&mut qc, Direction::Down)?;
            } else if last_at == last_line {
                self.scroll(&mut qc, Direction::Down)?;
            }
        }
        self.redraw_selected(&mut qc, true)?;
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
        // BUG:
        if let Some(num_new) = num_new {
            let new_comps = self.feed.items.iter().take(num_new).map(|i| i.build());
            self.textpad.components.push_front(new_comps);
            self.textpad.build();
            for _ in 0..num_new {
                self.move_select(&mut qc, Direction::Down)?;
            }
        }
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
        // FIXME: Add self.textpad.resize();
        self.draw(&mut qc)?;
        Ok(())
    }
}
