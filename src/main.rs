use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent},
    execute, queue,
    terminal::{self, ClearType},
};
use std::{io::Write, time::Duration};

static VERSION: &str = "0.0.1";
struct Output {
    win_size: (usize, usize),
    editor_contents: EditorContents,
    cursor_controller: CursorController,
}

impl Output {
    fn new() -> Self {
        let win_size = terminal::size()
            .map(|(x, y)| (x as usize, y as usize))
            .unwrap();
        let editor_contents = EditorContents::new();
        let cursor_controller = CursorController::new(win_size);
        Self {
            win_size,
            editor_contents,
            cursor_controller,
        }
    }

    fn draw_rows(&mut self) {
        for i in 0..self.win_size.1 {
            if i == self.win_size.1 / 3 {
                let mut welcome = format!("Pound Editor --- Version {}", VERSION);
                if welcome.len() > self.win_size.0 {
                    welcome.truncate(self.win_size.0)
                }

                let mut padding = (self.win_size.0 - welcome.len()) / 2;
                if padding != 0 {
                    self.editor_contents.push('~');
                    padding -= 1
                }
                (0..padding).for_each(|_| self.editor_contents.push(' '));
                self.editor_contents.push_str(&welcome);
            } else {
                self.editor_contents.push('~');
            }

            queue!(
                self.editor_contents,
                terminal::Clear(ClearType::UntilNewLine)
            )
            .unwrap();
            if i < self.win_size.1 - 1 {
                self.editor_contents.push_str("\r\n");
            }
        }
    }

    fn clear_screen() -> crossterm::Result<()> {
        execute!(std::io::stdout(), terminal::Clear(ClearType::All))
    }

    fn refresh_screen(&mut self) -> crossterm::Result<()> {
        queue!(
            self.editor_contents,
            cursor::Hide,
            terminal::Clear(ClearType::All),
            cursor::MoveTo(0, 0)
        )?;
        self.draw_rows();

        let cursor_x = self.cursor_controller.cursor_x;
        let cursor_y = self.cursor_controller.cursor_y;
        queue!(
            self.editor_contents,
            cursor::MoveTo(cursor_x as u16, cursor_y as u16),
            cursor::Show
        )?;
        self.editor_contents.flush()
    }

    fn move_cursor(&mut self, direction: KeyCode) {
        self.cursor_controller.move_cursor(direction);
    }
}

struct ClearUp;

impl Drop for ClearUp {
    fn drop(&mut self) {
        terminal::disable_raw_mode().expect("退出原始模式失败");
        Output::clear_screen().expect("Error");
    }
}

struct Reader;

impl Reader {
    fn read_key(&self) -> crossterm::Result<KeyEvent> {
        loop {
            if event::poll(Duration::from_millis(1000))? {
                if let Event::Key(event) = event::read()? {
                    return Ok(event);
                }
            }
        }
    }
}

struct Editor {
    reader: Reader,
    output: Output,
}

impl Editor {
    fn new() -> Self {
        Self {
            reader: Reader,
            output: Output::new(),
        }
    }

    fn process_keypress(&mut self) -> crossterm::Result<bool> {
        match self.reader.read_key()? {
            KeyEvent {
                code: KeyCode::Char('q'),
                modifiers: event::KeyModifiers::CONTROL,
            } => return Ok(false),

            KeyEvent {
                code:
                    direction @ (KeyCode::Up
                    | KeyCode::Down
                    | KeyCode::Left
                    | KeyCode::Right
                    | KeyCode::Home
                    | KeyCode::End),
                modifiers: event::KeyModifiers::NONE,
            } => self.output.move_cursor(direction),

            KeyEvent {
                code: val @ (KeyCode::PageUp | KeyCode::PageDown),
                modifiers: event::KeyModifiers::NONE,
            } => (0..self.output.win_size.1).for_each(|_| {
                self.output.move_cursor(if matches!(val, KeyCode::PageUp) {
                    KeyCode::Up
                } else {
                    KeyCode::Down
                });
            }),

            _ => {}
        }

        Ok(true)
    }

    fn run(&mut self) -> crossterm::Result<bool> {
        self.output.refresh_screen()?;
        self.process_keypress()
    }
}

struct EditorContents {
    contents: String,
}

impl EditorContents {
    fn new() -> Self {
        Self {
            contents: String::new(),
        }
    }

    fn push(&mut self, ch: char) {
        self.contents.push(ch)
    }

    fn push_str(&mut self, string: &str) {
        self.contents.push_str(string)
    }
}

impl std::io::Write for EditorContents {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match std::str::from_utf8(buf) {
            Ok(s) => {
                self.contents.push_str(s);
                Ok(s.len())
            }

            Err(_) => Err(std::io::ErrorKind::WriteZero.into()),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        let out = write!(std::io::stdout(), "{}", self.contents);
        std::io::stdout().flush()?;
        self.contents.clear();
        out
    }
}

struct CursorController {
    cursor_x: usize,
    cursor_y: usize,
    screen_columns: usize,
    screen_rows: usize,
}

impl CursorController {
    fn new(win_size: (usize, usize)) -> Self {
        Self {
            cursor_x: 0,
            cursor_y: 0,
            screen_columns: win_size.0,
            screen_rows: win_size.1,
        }
    }

    fn move_cursor(&mut self, direction: KeyCode) {
        match direction {
            KeyCode::Up => {
                self.cursor_y = self.cursor_y.saturating_sub(1);
            }
            KeyCode::Left => {
                if self.cursor_x != 0 {
                    self.cursor_x -= 1;
                }
            }
            KeyCode::Down => {
                if self.cursor_y != self.screen_rows - 1 {
                    self.cursor_y += 1;
                }
            }
            KeyCode::Right => {
                if self.cursor_x != self.screen_columns - 1 {
                    self.cursor_x += 1;
                }
            }
            KeyCode::End => self.cursor_x = self.screen_columns - 1,
            KeyCode::Home => self.cursor_x = 0,
            _ => unimplemented!(),
        }
    }
}

fn main() -> crossterm::Result<()> {
    let _clear_flag = ClearUp;
    terminal::enable_raw_mode()?;

    let mut editor = Editor::new();
    while editor.run()? {}

    Ok(())
    // loop {
    //     if event::poll(Duration::from_millis(500))? {
    //         if let Event::Key(event) = event::read()? {
    //             match event {
    //                 KeyEvent { code: KeyCode::Char('q'), modifiers: event::KeyModifiers::CONTROL } => break,

    //                 _ => {
    //                 }
    //             }

    //             println!("{:?}\r", event);
    //         }
    //     } else {
    //         println!("no input\r");
    //     }
    // }

    // 1
    // let mut buf = [0; 1];
    // while io::stdin().read(&mut buf).expect("read错误") == 1 && buf != [b'q'] {
    //     let char_str = buf[0] as char;
    //     if char_str.is_control() {
    //         println!("{}\r", char_str as u8);
    //     } else {
    //         println!("{}\r", char_str);
    //     }
    // }
}
