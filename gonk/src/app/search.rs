use super::Mode as AppMode;
use crate::widgets::{Cell, Row, Table, TableState};
use crossterm::event::KeyModifiers;
use gonk_core::{sqlite, Colors, Index};
use gonk_player::Player;
use tui::{
    backend::Backend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Span, Spans},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};

#[derive(Clone)]
pub enum Item {
    Song(Song),
    Album(Album),
    Artist(Artist),
}

#[derive(Clone, Default)]
pub struct Song {
    pub id: usize,
    pub name: String,
    pub album: String,
    pub artist: String,
}

#[derive(Clone, Default)]
pub struct Album {
    pub name: String,
    pub artist: String,
}

#[derive(Clone, Default)]
pub struct Artist {
    pub name: String,
}

pub enum Mode {
    Search,
    Select,
}

pub struct Search {
    query: String,
    prev_query: String,
    mode: Mode,
    results: Index<Item>,
    cache: Vec<Item>,
    colors: Colors,
}

impl Search {
    pub fn new(colors: Colors) -> Self {
        let mut s = Self {
            cache: Vec::new(),
            query: String::new(),
            prev_query: String::new(),
            mode: Mode::Search,
            results: Index::default(),
            colors,
        };
        //TODO: this is dumb
        s.update_engine();
        s.update_search();
        s
    }
    pub fn update_engine(&mut self) {
        let songs = sqlite::get_all_songs();
        let artists = sqlite::get_all_artists();
        let albums = sqlite::get_all_albums();
        self.cache = Vec::new();

        for (id, song) in songs {
            self.cache.push(Item::Song(Song {
                name: song.name,
                album: song.album,
                artist: song.artist,
                id,
            }));
        }

        for (album, artist) in albums {
            self.cache.push(Item::Album(Album {
                name: album,
                artist,
            }));
        }

        for artist in artists {
            self.cache.push(Item::Artist(Artist { name: artist }));
        }
    }
    pub fn update_search(&mut self) {
        if self.query == self.prev_query {
            return;
        } else {
            self.prev_query = self.query.clone();
        }

        let query = &self.query.to_lowercase();
        let mut results = Vec::new();

        for item in &self.cache {
            let acc = match item {
                Item::Song(song) => strsim::jaro_winkler(query, &song.name.to_lowercase()),
                Item::Album(album) => strsim::jaro_winkler(query, &album.name.to_lowercase()),
                Item::Artist(artist) => strsim::jaro_winkler(query, &artist.name.to_lowercase()),
            };

            if query.is_empty() && results.len() < 40 {
                results.push((item, acc));
            }

            if acc > 0.75 {
                results.push((item, acc));
            }
        }
        //TODO: sort self-titled albums bellow artists
        results.sort_by(|(_, a), (_, b)| b.partial_cmp(a).unwrap());
        self.results.data = results.into_iter().map(|(item, _)| item.clone()).collect();
    }
    pub fn on_key(&mut self, c: char) {
        if let Mode::Search = &self.mode {
            self.prev_query = self.query.clone();
            self.query.push(c);
        } else {
            match c {
                'k' => self.results.up(),
                'j' => self.results.down(),
                _ => (),
            }
        }
    }
    pub fn up(&mut self) {
        self.results.up();
    }
    pub fn down(&mut self) {
        self.results.down();
    }
    pub fn on_backspace(&mut self, modifiers: KeyModifiers) {
        match self.mode {
            Mode::Search => {
                if modifiers == KeyModifiers::CONTROL {
                    self.query.clear();
                } else {
                    self.query.pop();
                }
            }
            Mode::Select => {
                self.results.select(None);
                self.mode = Mode::Search;
            }
        }
    }
    pub fn on_escape(&mut self, mode: &mut AppMode) {
        match self.mode {
            Mode::Search => {
                if let Mode::Search = self.mode {
                    self.query.clear();
                    *mode = AppMode::Queue;
                }
            }
            Mode::Select => {
                self.mode = Mode::Search;
                self.results.select(None);
            }
        }
    }
    pub fn on_enter(&mut self, player: &mut Player) {
        match self.mode {
            Mode::Search => {
                if !self.results.is_empty() {
                    self.mode = Mode::Select;
                    self.results.select(Some(0));
                }
            }
            Mode::Select => {
                if let Some(item) = self.results.selected() {
                    let songs = match item {
                        Item::Song(song) => sqlite::get_songs_from_id(&[song.id]),
                        Item::Album(album) => {
                            sqlite::get_all_songs_from_album(&album.name, &album.artist)
                        }
                        Item::Artist(artist) => sqlite::get_songs_by_artist(&artist.name),
                    };

                    player.add_songs(songs);
                }
            }
        }
    }
}

impl Search {
    pub fn draw<B: Backend>(&self, f: &mut Frame<B>) {
        let area = f.size();

        let v = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Percentage(30),
                Constraint::Percentage(60),
            ])
            .split(area);

        let h = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(35), Constraint::Percentage(65)])
            .split(v[1]);

        self.draw_textbox(f, v[0]);

        let item = if self.results.selected().is_some() {
            self.results.selected()
        } else {
            self.results.data.first()
        };

        if let Some(item) = item {
            match item {
                Item::Song(song) => {
                    Search::song(f, &song.name, &song.album, &song.artist, h[0]);
                    self.album(f, &song.album, &song.artist, h[1]);
                }
                Item::Album(album) => {
                    self.album(f, &album.name, &album.artist, h[0]);
                    self.artist(f, &album.artist, h[1]);
                }
                Item::Artist(artist) => {
                    let albums = sqlite::get_all_albums_by_artist(&artist.name);

                    self.artist(f, &artist.name, h[0]);

                    let h_split = Layout::default()
                        .direction(Direction::Horizontal)
                        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                        .split(h[1]);

                    //draw the first two albums
                    for (i, area) in h_split.iter().enumerate() {
                        if let Some(album) = albums.get(i) {
                            self.album(f, album, &artist.name, *area);
                        }
                    }
                }
            }
            self.draw_results(f, v[2]);
        } else {
            self.draw_results(f, v[1].union(v[2]));
        }

        self.update_cursor(f);
    }
    fn song<B: Backend>(f: &mut Frame<B>, name: &str, album: &str, artist: &str, area: Rect) {
        let song_table = Table::new(vec![
            Row::new(vec![Spans::from(Span::raw(album))]),
            Row::new(vec![Spans::from(Span::raw(artist))]),
        ])
        .header(
            Row::new(vec![Span::styled(
                format!("{} ", name),
                Style::default().add_modifier(Modifier::ITALIC),
            )])
            .bottom_margin(1),
        )
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title("Song"),
        )
        .widths(&[Constraint::Percentage(100)]);

        f.render_widget(song_table, area);
    }
    fn album<B: Backend>(&self, f: &mut Frame<B>, album: &str, artist: &str, area: Rect) {
        let cells: Vec<_> = sqlite::get_all_songs_from_album(album, artist)
            .iter()
            .map(|song| Row::new(vec![Cell::from(format!("{}. {}", song.number, song.name))]))
            .collect();

        let album_table = Table::new(cells)
            .header(
                Row::new(vec![Cell::from(Span::styled(
                    format!("{} ", album),
                    Style::default().add_modifier(Modifier::ITALIC),
                ))])
                .bottom_margin(1),
            )
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .title("Album"),
            )
            .widths(&[Constraint::Percentage(100)]);

        f.render_widget(album_table, area);
    }
    fn artist<B: Backend>(&self, f: &mut Frame<B>, artist: &str, area: Rect) {
        let albums = sqlite::get_all_albums_by_artist(artist);
        let cells: Vec<_> = albums
            .iter()
            .map(|album| Row::new(vec![Cell::from(Span::raw(album))]))
            .collect();

        let artist_table = Table::new(cells)
            .header(
                Row::new(vec![Cell::from(Span::styled(
                    format!("{} ", artist),
                    Style::default().add_modifier(Modifier::ITALIC),
                ))])
                .bottom_margin(1),
            )
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .title("Artist"),
            )
            .widths(&[Constraint::Percentage(100)]);

        f.render_widget(artist_table, area);
    }
    fn draw_results<B: Backend>(&self, f: &mut Frame<B>, area: Rect) {
        let get_cell = |item: &Item, selected: bool| -> Row {
            let selected_cell = if selected {
                Cell::from(">")
            } else {
                Cell::default()
            };

            match item {
                Item::Song(song) => {
                    let song = sqlite::get_songs_from_id(&[song.id])[0].clone();
                    Row::new(vec![
                        selected_cell,
                        Cell::from(song.name).style(Style::default().fg(self.colors.title)),
                        Cell::from(song.album).style(Style::default().fg(self.colors.album)),
                        Cell::from(song.artist).style(Style::default().fg(self.colors.artist)),
                    ])
                }
                Item::Album(album) => Row::new(vec![
                    selected_cell,
                    Cell::from(format!("{} - Album", album.name))
                        .style(Style::default().fg(self.colors.title)),
                    Cell::from("").style(Style::default().fg(self.colors.album)),
                    Cell::from(album.artist.clone()).style(Style::default().fg(self.colors.artist)),
                ]),
                Item::Artist(artist) => Row::new(vec![
                    selected_cell,
                    Cell::from(format!("{} - Artist", artist.name))
                        .style(Style::default().fg(self.colors.title)),
                    Cell::from("").style(Style::default().fg(self.colors.album)),
                    Cell::from("").style(Style::default().fg(self.colors.artist)),
                ]),
            }
        };

        let selected = &self.results.selection();
        let rows: Vec<_> = self
            .results
            .data
            .iter()
            .enumerate()
            .map(|(i, item)| {
                if let Some(s) = selected {
                    if s == &i {
                        return get_cell(item, true);
                    }
                } else if i == 0 {
                    return get_cell(item, false);
                }
                get_cell(item, false)
            })
            .collect();

        let italic = Style::default().add_modifier(Modifier::ITALIC);
        let t = Table::new(rows)
            .header(
                Row::new(vec![
                    Cell::default(),
                    Cell::from("Name").style(italic),
                    Cell::from("Album").style(italic),
                    Cell::from("Artist").style(italic),
                ])
                .bottom_margin(1),
            )
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded),
            )
            .widths(&[
                Constraint::Length(1),
                Constraint::Percentage(40),
                Constraint::Percentage(40),
                Constraint::Percentage(20),
            ]);

        let mut state = TableState::new(self.results.selection());
        f.render_stateful_widget(t, area, &mut state);
    }
    fn draw_textbox<B: Backend>(&self, f: &mut Frame<B>, area: Rect) {
        let p = Paragraph::new(self.query.as_ref())
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded),
            )
            .alignment(Alignment::Left);
        f.render_widget(p, area);
    }
    fn update_cursor<B: Backend>(&self, f: &mut Frame<B>) {
        let area = f.size();
        //Move the cursor position when typing
        if let Mode::Search = self.mode {
            if self.results.is_none() && self.query.is_empty() {
                f.set_cursor(1, 1);
            } else {
                //TODO: casting to u16 could fail
                let mut len = self.query.len() as u16;
                if len > area.width {
                    len = area.width;
                }
                f.set_cursor(len + 1, 1);
            }
        }
    }
}
