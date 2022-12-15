use crate::widgets::{List, ListItem, ListState};
use crate::{Frame, Widget, VDB};
use crossterm::event::MouseEvent;
use gonk_core::{profile, vdb, StaticIndex};
use gonk_core::{Album, Index, Song};
use tui::{
    layout::{Constraint, Direction, Layout, Rect},
    widgets::{Block, BorderType, Borders},
};

#[derive(PartialEq, Eq)]
pub enum Mode {
    Artist,
    Album,
    Song,
}

pub struct Browser {
    artists: Index<&'static String>,
    albums: StaticIndex<Album>,
    ///Title, (disc, number)
    songs: Index<(String, (u8, u8))>,
    pub mode: Mode,
}

impl Browser {
    pub fn new() -> Self {
        let artists = Index::new(unsafe { vdb::artists(&VDB) }, Some(0));
        let mut albums: StaticIndex<Album> = StaticIndex::default();
        let mut songs = Index::default();

        if let Some(artist) = artists.selected() {
            albums = StaticIndex::new(unsafe { vdb::albums_by_artist(&VDB, artist).unwrap() });

            if let Some(album) = albums.selected() {
                songs = Index::new(
                    album
                        .songs
                        .iter()
                        .map(|song| {
                            (
                                format!("{}. {}", song.track_number, song.title),
                                (song.disc_number, song.track_number),
                            )
                        })
                        .collect(),
                    Some(0),
                );
            }
        }

        Self {
            artists,
            albums,
            songs,
            mode: Mode::Artist,
        }
    }
}

impl Widget for Browser {
    fn up(&mut self) {
        profile!();
        match self.mode {
            Mode::Artist => self.artists.up(),
            Mode::Album => self.albums.up(),
            Mode::Song => self.songs.up(),
        }
        update(self);
    }

    fn down(&mut self) {
        profile!();
        match self.mode {
            Mode::Artist => self.artists.down(),
            Mode::Album => self.albums.down(),
            Mode::Song => self.songs.down(),
        }
        update(self);
    }

    fn left(&mut self) {
        match self.mode {
            Mode::Artist => (),
            Mode::Album => self.mode = Mode::Artist,
            Mode::Song => self.mode = Mode::Album,
        }
    }

    fn right(&mut self) {
        match self.mode {
            Mode::Artist => self.mode = Mode::Album,
            Mode::Album => self.mode = Mode::Song,
            Mode::Song => (),
        }
    }

    fn draw(&mut self, f: &mut Frame, area: Rect, mouse_event: Option<MouseEvent>) {
        draw(self, area, f, mouse_event);
    }
}

pub fn refresh(browser: &mut Browser) {
    browser.mode = Mode::Artist;

    browser.artists = Index::new(unsafe { vdb::artists(&VDB) }, Some(0));
    browser.albums = StaticIndex::default();
    browser.songs = Index::default();

    update_albums(browser);
}

pub fn update(browser: &mut Browser) {
    match browser.mode {
        Mode::Artist => update_albums(browser),
        Mode::Album => update_songs(browser),
        Mode::Song => (),
    }
}

pub fn update_albums(browser: &mut Browser) {
    //Update the album based on artist selection
    if let Some(artist) = browser.artists.selected() {
        let albums = unsafe { vdb::albums_by_artist(&VDB, artist).unwrap() };
        browser.albums = StaticIndex::new(albums);
        update_songs(browser);
    }
}

pub fn update_songs(browser: &mut Browser) {
    if let Some(artist) = browser.artists.selected() {
        if let Some(album) = browser.albums.selected() {
            let songs = unsafe { vdb::album(&VDB, artist, &album.title).unwrap() }
                .songs
                .iter()
                .map(|song| {
                    (
                        format!("{}. {}", song.track_number, song.title),
                        (song.disc_number, song.track_number),
                    )
                })
                .collect();
            browser.songs = Index::new(songs, Some(0));
        }
    }
}

pub fn get_selected(browser: &Browser) -> Vec<&'static Song> {
    if let Some(artist) = browser.artists.selected() {
        if let Some(album) = browser.albums.selected() {
            if let Some((_, (disc, number))) = browser.songs.selected() {
                return match browser.mode {
                    Mode::Artist => {
                        let albums = unsafe { vdb::artist(&VDB, artist).unwrap() };
                        let mut songs = Vec::new();
                        for album in albums {
                            songs.extend(&album.songs);
                        }
                        songs
                    }
                    Mode::Album => {
                        let album = unsafe { vdb::album(&VDB, artist, &album.title).unwrap() };
                        let mut songs = Vec::new();
                        for song in &album.songs {
                            songs.push(song);
                        }
                        songs
                    }
                    Mode::Song => {
                        vec![unsafe {
                            vdb::song(&VDB, artist, &album.title, *disc, *number).unwrap()
                        }]
                    }
                };
            }
        }
    }
    Vec::new()
}

pub fn draw(browser: &mut Browser, area: Rect, f: &mut Frame, event: Option<MouseEvent>) {
    profile!();
    let size = area.width / 3;
    let rem = area.width % 3;

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(size),
            Constraint::Length(size),
            Constraint::Length(size + rem),
        ])
        .split(area);

    if let Some(event) = event {
        let rect = Rect {
            x: event.column,
            y: event.row,
            ..Default::default()
        };
        if rect.intersects(chunks[2]) {
            browser.mode = Mode::Song;
        } else if rect.intersects(chunks[1]) {
            browser.mode = Mode::Album;
        } else if rect.intersects(chunks[0]) {
            browser.mode = Mode::Artist;
        }
    }

    let a: Vec<ListItem> = browser
        .artists
        .iter()
        .map(|name| ListItem::new(name.as_str()))
        .collect();

    let b: Vec<ListItem> = browser
        .albums
        .iter()
        .map(|name| ListItem::new(name.title.as_str()))
        .collect();

    let c: Vec<ListItem> = browser
        .songs
        .iter()
        .map(|(name, _)| ListItem::new(name.as_str()))
        .collect();

    let artists = list("─Aritst", &a, browser.mode == Mode::Artist);
    let albums = list("─Album", &b, browser.mode == Mode::Album);
    let songs = list("─Song", &c, browser.mode == Mode::Song);

    f.render_stateful_widget(
        artists,
        chunks[0],
        &mut ListState::new(browser.artists.index()),
    );
    f.render_stateful_widget(
        albums,
        chunks[1],
        &mut ListState::new(browser.albums.index()),
    );
    f.render_stateful_widget(songs, chunks[2], &mut ListState::new(browser.songs.index()));
}

fn list<'a>(title: &'static str, content: &'a [ListItem], use_symbol: bool) -> List<'a> {
    let list = List::new(content).block(
        Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded),
    );

    if use_symbol {
        list.highlight_symbol(">")
    } else {
        list.highlight_symbol("")
    }
}
