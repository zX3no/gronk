use std::{
    cell::RefCell,
    rc::Rc,
    sync::{Arc, Mutex},
};

use crate::widget::{List, ListItem, ListState};
use gonk_tcp::Client;
use gonk_types::Index;
use tui::{
    backend::Backend,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Style},
    text::Spans,
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};

pub enum Mode {
    Artist,
    Album,
    Song,
}

impl Mode {
    pub fn next(&mut self) {
        match self {
            Mode::Artist => *self = Mode::Album,
            Mode::Album => *self = Mode::Song,
            Mode::Song => (),
        }
    }
    pub fn prev(&mut self) {
        match self {
            Mode::Artist => (),
            Mode::Album => *self = Mode::Artist,
            Mode::Song => *self = Mode::Album,
        }
    }
}

pub struct Browser {
    pub mode: Mode,
    pub artists: Option<usize>,
    pub albums: Option<usize>,
    pub songs: Option<usize>,
    client: Rc<RefCell<Client>>,
}

impl Browser {
    pub fn new(client: Rc<RefCell<Client>>) -> Self {
        optick::event!("new browser");

        Self {
            mode: Mode::Artist,
            artists: None,
            albums: None,
            songs: None,
            client,
        }
    }
    pub fn on_enter(&self) {
        todo!();
    }
    pub fn prev(&mut self) {
        self.mode.prev();
    }
    pub fn next(&mut self) {
        self.mode.prev();
    }
    pub fn up(&mut self) {
        // match self.mode {
        //     Mode::Artist => self.artists.up(),
        //     Mode::Album => self.albums.up(),
        //     Mode::Song => self.songs.up(),
        // }
    }
    pub fn down(&mut self) {
        // match self.mode {
        //     Mode::Artist => self.artists.down(),
        //     Mode::Album => self.albums.down(),
        //     Mode::Song => self.songs.down(),
        // }
    }
    pub fn update(&mut self) {
        // self.client.update_albums(self.artists);
        // self.client.update_songs(self.albums, self.artists);
    }
    pub fn draw<B: Backend>(&self, f: &mut Frame<B>) {
        optick::event!("draw Browser");
        self.draw_browser(f);
        // Browser::draw_popup(f);
    }
    pub fn draw_browser<B: Backend>(&self, f: &mut Frame<B>) {
        let area = f.size();

        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(
                [
                    Constraint::Percentage(33),
                    Constraint::Percentage(33),
                    Constraint::Percentage(33),
                ]
                .as_ref(),
            )
            .split(area);

        let client = self.client.borrow();
        let a: Vec<_> = client
            .artists
            .iter()
            .map(|name| ListItem::new(name.as_str()))
            .collect();

        let b: Vec<_> = client
            .albums
            .iter()
            .map(|name| ListItem::new(name.as_str()))
            .collect();

        let c: Vec<_> = client
            .songs
            .iter()
            .map(|song| ListItem::new(format!("{}. {}", song.0, song.1)))
            .collect();

        let artists = List::new(a)
            .block(
                Block::default()
                    .title("─Aritst")
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded),
            )
            .style(Style::default().fg(Color::White))
            .highlight_style(Style::default())
            .highlight_symbol(">");

        let mut artist_state = ListState::new(self.artists);

        let albums = List::new(b)
            .block(
                Block::default()
                    .title("─Album")
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded),
            )
            .style(Style::default().fg(Color::White))
            .highlight_style(Style::default())
            .highlight_symbol(">");

        let mut album_state = ListState::new(self.albums);

        let songs = List::new(c)
            .block(
                Block::default()
                    .title("─Song")
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded),
            )
            .style(Style::default().fg(Color::White))
            .highlight_style(Style::default())
            .highlight_symbol(">");

        let mut song_state = ListState::new(self.songs);

        //TODO: better way of doing this?
        match self.mode {
            Mode::Artist => {
                album_state.select(None);
                song_state.select(None);
            }
            Mode::Album => {
                artist_state.select(None);
                song_state.select(None);
            }
            Mode::Song => {
                artist_state.select(None);
                album_state.select(None);
            }
        }

        f.render_stateful_widget(artists, chunks[0], &mut artist_state);
        f.render_stateful_widget(albums, chunks[1], &mut album_state);
        f.render_stateful_widget(songs, chunks[2], &mut song_state);
    }
}
