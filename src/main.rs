use std::io::{stdout, Stdout};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use crossterm::{
    cursor,
    event::{read, Event, KeyCode, KeyEvent},
    execute,
    style::Print,
    terminal::{self, ClearType},
    tty::IsTty,
    Result,
};

enum State {
    Pause,
    Stop,
    Resume,
    NewMessage(String),
}

struct CharGen {
    position: usize,
    chars: Vec<char>,
}

impl CharGen {
    pub fn new(chars: &str) -> Self {
        let chars: Vec<char> = chars.chars().collect();
        CharGen { position: 0, chars }
    }
    pub fn update(&mut self, newmsg: &str) {
        self.chars.clear();
        let newvec: Vec<char> = newmsg.chars().collect();
        self.chars.extend(newvec);
        self.position = self.position % self.chars.len();
    }
}

impl Iterator for CharGen {
    type Item = char;

    fn next(&mut self) -> Option<Self::Item> {
        let result = Some(self.chars[self.position]);
        let size = self.chars.len();
        self.position = (self.position + 1) % size;
        result
    }
}

fn main() -> Result<()> {
    let mut out = stdout();
    // to accept typing Esc key
    terminal::enable_raw_mode()?;
    // clear terminal and print help messege
    execute!(
        out,
        terminal::Clear(ClearType::All),
        cursor::MoveTo(1, 2),
        Print("Type strngs. Trace edges by Enter key. Stop by Esc key.")
    )?;
    // _support_check(&out);

    let (tx, rx) = mpsc::channel::<State>();
    let _current_position = cursor::position().unwrap(); // unused

    let join_handle = thread::spawn(move || -> Result<()> {
        let (width, height) = terminal::size().unwrap();
        let mut cg = CharGen::new("hello world.");
        let should_pause = |rx: &mpsc::Receiver<State>| match rx.try_recv() {
            Ok(State::Pause) => true,
            _ => false,
        };
        loop {
            // first state is Pause
            match rx.recv() {
                Ok(State::NewMessage(msg)) => {
                    cg.update(&msg);
                    continue;
                }
                Ok(State::Resume) => break,
                Ok(State::Stop) => return Ok(()),
                _ => continue,
            }
        }
        loop {
            for i in 0..2 * (width + height) {
                if should_pause(&rx) {
                    // wait for changing state
                    loop {
                        match rx.recv() {
                            Ok(State::NewMessage(msg)) => {
                                cg.update(&msg);
                                continue;
                            }
                            Ok(State::Resume) => break,
                            Ok(State::Stop) => return Ok(()),
                            _ => continue,
                        }
                    }
                }
                // calc guruguru char position
                let (x, y) = match i {
                    i if i < width => (i, 0),                                          // top
                    i if width <= i && i < (width + height) => (width - 1, i - width), // right
                    i if (width + height) <= i && i < (2 * width + height) => {
                        (2 * width + height - i - 1, height - 1) // bottom
                    }
                    _ => (0, 2 * (width + height) - i - 1), // left
                };
                execute!(
                    out,
                    cursor::Hide,
                    cursor::MoveTo(x, y),
                    Print(cg.next().unwrap())
                )?;
                thread::sleep(Duration::from_millis(10));
            }
        }
    });

    // read input event
    let mut out = stdout();
    let mut msg: Vec<char> = Vec::new();
    loop {
        let event = read().unwrap();
        match event {
            Event::Key(KeyEvent { code, .. }) => {
                _ = tx.send(State::Pause);
                let (width, _) = terminal::size().unwrap();
                let inner_width: usize = (width - 2) as usize;

                // clear 1st line (inside)
                match code {
                    KeyCode::Char(ch) => {
                        if msg.len() + 1 < inner_width {
                            msg.push(ch);
                        }
                    }
                    KeyCode::Enter => {
                        // start tracing terminal edges by enter key
                        if msg.is_empty() {
                            let defaultchars: Vec<char> = "Hello World ".chars().collect();
                            msg.extend(defaultchars);
                        }
                        let newmessage: String = msg.clone().into_iter().collect();
                        _ = tx.send(State::NewMessage(newmessage));
                        msg.clear();
                        _ = tx.send(State::Resume);
                        continue;
                    }
                    KeyCode::Esc => {
                        // stop by esc key
                        execute!(out, cursor::MoveTo(1, 1), Print("prepare for exiting..."))?;
                        _ = tx.send(State::Stop);
                        _ = join_handle.join();
                        break;
                    }
                    KeyCode::Backspace => {
                        msg.pop();
                    }
                    _ => {
                        let newchars: Vec<char> = format!("{:?}", code).chars().collect();
                        if msg.len() + newchars.len() < inner_width {
                            msg.extend(newchars);
                        }
                    }
                }
                // update terminal for a message
                // clear 1 line
                _ = execute!(out, cursor::MoveTo(1, 1), Print(" ".repeat(inner_width)));

                let s: String = msg.iter().take(inner_width).collect();

                // show msg
                execute!(out, cursor::MoveTo(1, 1), Print(s))?;
            }
            _ => (),
        }
    }

    // clear all changes
    execute!(
        out,
        terminal::Clear(ClearType::All),
        cursor::MoveTo(0, 0),
        cursor::Show
    )?;
    terminal::disable_raw_mode()?;
    Ok(())
}

fn _support_check(out: &Stdout) {
    if out.is_tty() {
        println!("ANSI escape sequences are supported");
    } else {
        println!("ANSI escape sequences are not supported");
    }
}
