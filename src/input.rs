use crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers, MouseEventKind};

pub enum Direction {
    Up,
    Down,
    ThreeUp,
    ThreeDown,
}

pub enum Controls {
    Quit,
    Resize((u16, u16)),
    MoveSelect(Direction),
    Scroll(Direction),
    Select,
    GotoTop,
}

pub enum View {
    Feed,
    Article,
}

pub struct InputBuffer {
    char_buffer: Vec<char>,
}

impl InputBuffer {
    pub fn new() -> Self {
        Self {
            char_buffer: vec![],
        }
    }

    fn map_key(&mut self, c: char, view: View) -> Option<Controls> {
        self.char_buffer.push(c);
        let control = match (self.char_buffer.as_slice(), view) {
            (['k'], View::Feed) => Some(Controls::MoveSelect(Direction::Up)),
            (['k'], View::Article) => Some(Controls::Scroll(Direction::Up)),
            (['j'], View::Feed) => Some(Controls::MoveSelect(Direction::Down)),
            (['j'], View::Article) => Some(Controls::Scroll(Direction::Down)),
            (['q'], _) => Some(Controls::Quit),
            (['g', 'g'], _) => Some(Controls::GotoTop),
            // TODO: Consider adding Controls::GotoBottom
            _ => None,
        };
        if control.is_some() {
            self.char_buffer.clear();
        }
        return control;
    }

    pub fn map(&mut self, event: Event, view: View) -> Option<Controls> {
        return match event {
            Event::Key(event) => {
                if event.kind == KeyEventKind::Press {
                    match (event.code, view) {
                        (KeyCode::Char(c), view) => {
                            if event.modifiers.contains(KeyModifiers::CONTROL) && c == 'c' {
                                Some(Controls::Quit)
                            } else {
                                self.map_key(c, view)
                            }
                        }
                        (KeyCode::Enter, View::Feed) => Some(Controls::Select),
                        (KeyCode::Backspace, _) => Some(Controls::Quit),
                        (KeyCode::Up, View::Feed) => Some(Controls::MoveSelect(Direction::Up)),
                        (KeyCode::Up, View::Article) => Some(Controls::Scroll(Direction::Up)),
                        (KeyCode::Down, View::Feed) => Some(Controls::MoveSelect(Direction::Down)),
                        (KeyCode::Down, View::Article) => Some(Controls::Scroll(Direction::Down)),
                        _ => None,
                    }
                } else {
                    None
                }
            }
            Event::Resize(w, h) => Some(Controls::Resize((w, h))),
            Event::Mouse(event) => match (event.kind, view) {
                (MouseEventKind::ScrollUp, View::Article) => {
                    Some(Controls::Scroll(Direction::ThreeUp))
                }
                (MouseEventKind::ScrollDown, View::Article) => {
                    Some(Controls::Scroll(Direction::ThreeDown))
                }
                (MouseEventKind::ScrollUp, View::Feed) => Some(Controls::MoveSelect(Direction::Up)),
                (MouseEventKind::ScrollDown, View::Feed) => {
                    Some(Controls::MoveSelect(Direction::Down))
                }
                // TODO: Add mouse select
                _ => None,
            },
            _ => None,
        };
    }
}
