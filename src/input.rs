use crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers, MouseButton, MouseEventKind};

pub enum Direction {
    Up,
    Down,
}

pub enum Controls {
    Quit,
    Resize((u16, u16)),
    MoveSelect(Direction),
    Scroll(Direction, u16),
    Select,
    MouseSelect(u16, u16),
    GotoTop,
    Refresh,
}

pub enum View {
    Feed,
    Article,
    Error,
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

    pub fn clear(&mut self) {
        self.char_buffer.clear();
    }

    fn map_key(&mut self, c: char, view: View) -> Option<Controls> {
        self.char_buffer.push(c);
        let control = match (self.char_buffer.as_slice(), view) {
            (['k'], View::Feed) => Some(Controls::MoveSelect(Direction::Up)),
            (['k'], View::Article) => Some(Controls::Scroll(Direction::Up, 1)),
            (['j'], View::Feed) => Some(Controls::MoveSelect(Direction::Down)),
            (['j'], View::Article) => Some(Controls::Scroll(Direction::Down, 1)),
            (['q'], _) => Some(Controls::Quit),
            (['g', 'g'], _) => Some(Controls::GotoTop),
            (['r'], View::Feed) => Some(Controls::Refresh),
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
                        (KeyCode::Up, View::Article) => Some(Controls::Scroll(Direction::Up, 1)),
                        (KeyCode::Down, View::Feed) => Some(Controls::MoveSelect(Direction::Down)),
                        (KeyCode::Down, View::Article) => {
                            Some(Controls::Scroll(Direction::Down, 1))
                        }
                        _ => None,
                    }
                } else {
                    None
                }
            }
            Event::Resize(w, h) => Some(Controls::Resize((w, h))),
            Event::Mouse(event) => match (event.kind, view) {
                (MouseEventKind::ScrollUp, View::Article) => {
                    Some(Controls::Scroll(Direction::Up, 3))
                }
                (MouseEventKind::ScrollDown, View::Article) => {
                    Some(Controls::Scroll(Direction::Down, 3))
                }
                (MouseEventKind::ScrollUp, View::Feed) => Some(Controls::MoveSelect(Direction::Up)),
                (MouseEventKind::ScrollDown, View::Feed) => {
                    Some(Controls::MoveSelect(Direction::Down))
                }
                (MouseEventKind::Down(MouseButton::Left), View::Feed) => {
                    Some(Controls::MouseSelect(event.column, event.row))
                }
                (MouseEventKind::Down(MouseButton::Right), _) => Some(Controls::Quit),
                _ => None,
            },
            _ => None,
        };
    }
}
