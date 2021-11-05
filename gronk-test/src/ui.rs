use tui::backend::Backend;
use tui::layout::*;
use tui::style::*;
use tui::widgets::*;
use tui::Frame;

use crate::app::App;
use crate::app::Mode;

pub fn draw<B: Backend>(f: &mut Frame<B>, app: &mut App) {
    if let Mode::Queue = app.ui_mode {
        return;
    }
    let area = f.size();

    let music = &mut app.music;

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            [
                Constraint::Ratio(1, 3),
                Constraint::Ratio(1, 3),
                Constraint::Ratio(1, 3),
                Constraint::Length(1),
            ]
            .as_ref(),
        )
        .margin(1)
        .split(area);

    let a: Vec<_> = music
        .artist_names()
        .iter()
        .map(|name| ListItem::new(name.as_str()))
        .collect();

    let b: Vec<_> = music
        .album_names()
        .iter()
        .map(|name| ListItem::new(name.as_str()))
        .collect();

    let c: Vec<_> = music
        .song_names()
        .iter()
        .map(|name| ListItem::new(name.as_str()))
        .collect();

    let artists = List::new(a)
        .block(Block::default().title("Aritst").borders(Borders::ALL))
        .style(Style::default().fg(Color::White))
        .highlight_style(Style::default())
        .highlight_symbol("> ");

    let mut artist_state = ListState::default();
    artist_state.select(music.get_selected_artist());

    let albums = List::new(b)
        .block(Block::default().title("Album").borders(Borders::ALL))
        .style(Style::default().fg(Color::White))
        .highlight_style(Style::default())
        .highlight_symbol("> ");

    let mut album_state = ListState::default();
    album_state.select(music.get_selected_album());

    let songs = List::new(c)
        .block(Block::default().title("Song").borders(Borders::ALL))
        .style(Style::default().fg(Color::White))
        .highlight_style(Style::default())
        .highlight_symbol("> ");

    let mut song_state = ListState::default();
    song_state.select(music.get_selected_song());

    let no_state = ListState::default();

    match app.browser_mode {
        crate::app::BrowserMode::Artist => {
            album_state = no_state.clone();
            song_state = no_state;
        }
        crate::app::BrowserMode::Album => {
            artist_state = no_state.clone();
            song_state = no_state;
        }
        crate::app::BrowserMode::Song => {
            artist_state = no_state.clone();
            album_state = no_state;
        }
    }

    f.render_stateful_widget(artists, chunks[0], &mut artist_state);
    f.render_stateful_widget(albums, chunks[1], &mut album_state);
    f.render_stateful_widget(songs, chunks[2], &mut song_state);
}
