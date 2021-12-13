use app::App;
use std::{
    io::stdout,
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

use crossterm::{
    event::{
        self, EnableMouseCapture, Event as CTEvent, KeyCode, KeyEvent, KeyModifiers, MouseEvent,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use tui::{backend::CrosstermBackend, Terminal};

mod app;
mod index;
mod modes;
mod ui;

enum Event {
    Input(KeyEvent),
    MouseInput(MouseEvent),
    Tick,
}

fn main() {
    let orig_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        execute!(stdout(), LeaveAlternateScreen).unwrap();
        orig_hook(panic_info);
        std::process::exit(1);
    }));

    execute!(stdout(), EnterAlternateScreen, EnableMouseCapture).unwrap();
    enable_raw_mode().unwrap();

    let backend = CrosstermBackend::new(stdout());
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.clear().unwrap();
    terminal.hide_cursor().unwrap();

    let mut app = App::new();

    let (tx, rx) = mpsc::channel();
    let tick_rate = Duration::from_millis(50);

    thread::spawn(move || {
        let mut last_tick = Instant::now();
        loop {
            // poll for tick rate duration, if no events, sent tick event.
            let timeout = tick_rate
                .checked_sub(last_tick.elapsed())
                .unwrap_or_else(|| Duration::from_secs(0));

            if event::poll(timeout).unwrap() {
                match event::read().unwrap() {
                    CTEvent::Key(key) => tx.send(Event::Input(key)).unwrap(),
                    CTEvent::Mouse(mouse) => tx.send(Event::MouseInput(mouse)).unwrap(),
                    _ => (),
                }
            }

            if last_tick.elapsed() >= tick_rate {
                tx.send(Event::Tick).unwrap();
                last_tick = Instant::now();
            }
        }
    });

    loop {
        terminal.draw(|f| ui::draw(f, &mut app)).unwrap();
        match rx.recv().unwrap() {
            Event::Input(event) => {
                if let KeyCode::Char('c') = event.code {
                    if event.modifiers == KeyModifiers::CONTROL {
                        disable_raw_mode().unwrap();
                        execute!(terminal.backend_mut(), LeaveAlternateScreen).unwrap();
                        terminal.show_cursor().unwrap();
                        break;
                    } else {
                        app.handle_char('c', event.modifiers);
                    }
                } else {
                    app.input(event.code, event.modifiers);
                }
            }
            Event::Tick => {
                app.on_tick();
            }
            Event::MouseInput(event) => {
                app.mouse(event);
            }
        }
    }
}
