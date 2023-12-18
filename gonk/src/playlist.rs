use crate::{ALBUM, ARTIST, TITLE};
use gonk_core::{Index, Song};
use std::{error::Error, mem};
use winter::*;

#[derive(PartialEq, Eq)]
pub enum Mode {
    Playlist,
    Song,
    Popup,
}

pub struct Playlist {
    pub mode: Mode,
    pub lists: Index<gonk_core::Playlist>,
    pub song_buffer: Vec<Song>,
    pub search_query: String,
    pub search_result: String,
    pub changed: bool,
    pub delete: bool,
    pub yes: bool,
}

impl Playlist {
    pub fn new() -> std::result::Result<Self, Box<dyn Error>> {
        Ok(Self {
            mode: Mode::Playlist,
            lists: Index::from(gonk_core::playlist::playlists()),
            song_buffer: Vec::new(),
            changed: false,
            search_query: String::new(),
            search_result: String::from("Enter a playlist name..."),
            delete: false,
            yes: true,
        })
    }
}

pub fn up(playlist: &mut Playlist) {
    if !playlist.delete {
        match playlist.mode {
            Mode::Playlist => {
                playlist.lists.up();
            }
            Mode::Song => {
                if let Some(selected) = playlist.lists.selected_mut() {
                    selected.songs.up();
                }
            }
            Mode::Popup => (),
        }
    }
}

pub fn down(playlist: &mut Playlist) {
    if !playlist.delete {
        match playlist.mode {
            Mode::Playlist => {
                playlist.lists.down();
            }
            Mode::Song => {
                if let Some(selected) = playlist.lists.selected_mut() {
                    selected.songs.down();
                }
            }
            Mode::Popup => (),
        }
    }
}

pub fn left(playlist: &mut Playlist) {
    if playlist.delete {
        playlist.yes = true;
    } else if let Mode::Song = playlist.mode {
        playlist.mode = Mode::Playlist;
    }
}

pub fn right(playlist: &mut Playlist) {
    if playlist.delete {
        playlist.yes = false;
    } else {
        match playlist.mode {
            Mode::Playlist if playlist.lists.selected().is_some() => playlist.mode = Mode::Song,
            _ => (),
        }
    }
}

pub const fn centered_rect(width: u16, height: u16, area: Rect) -> Option<Rect> {
    let v = area.height / 2;
    let h = area.width / 2;
    let mut rect = area.inner((v.saturating_sub(height / 2), h.saturating_sub(width / 2)));
    rect.width = width;
    rect.height = height;
    if area.height < rect.height || area.width < rect.width {
        None
    } else {
        Some(rect)
    }
}

pub fn draw(
    playlist: &mut Playlist,
    area: winter::Rect,
    buf: &mut winter::Buffer,
    mouse: Option<(u16, u16)>,
) -> Option<(u16, u16)> {
    let horizontal = layout(
        area,
        Direction::Horizontal,
        &[Constraint::Percentage(30), Constraint::Percentage(70)],
    );

    if let Some((x, y)) = mouse {
        let rect = Rect {
            x,
            y,
            ..Default::default()
        };
        if rect.intersects(horizontal[1]) {
            playlist.mode = Mode::Song;
        } else if rect.intersects(horizontal[0]) {
            playlist.mode = Mode::Playlist;
        }
    }

    let items: Vec<Lines<'_>> = playlist.lists.iter().map(|p| lines!(p.name())).collect();
    let symbol = if let Mode::Playlist = playlist.mode {
        ">"
    } else {
        ""
    };

    list(&items)
        .block(block().title("Playlist").margin(1))
        .symbol(symbol)
        .draw(horizontal[0], buf, playlist.lists.index());

    let song_block = block().title("Songs").margin(1);
    if let Some(selected) = playlist.lists.selected() {
        let rows: Vec<_> = selected
            .songs
            .iter()
            .map(|song| {
                row![
                    song.title.as_str().fg(TITLE),
                    song.album.as_str().fg(ALBUM),
                    song.artist.as_str().fg(ARTIST)
                ]
            })
            .collect();

        let symbol = if playlist.mode == Mode::Song { ">" } else { "" };
        let table = table(
            rows,
            &[
                Constraint::Percentage(42),
                Constraint::Percentage(30),
                Constraint::Percentage(28),
            ],
        )
        .symbol(symbol)
        .block(song_block);
        table.draw(horizontal[1], buf, selected.songs.index());
    } else {
        song_block.draw(horizontal[1], buf);
    }

    if playlist.delete {
        if let Some(area) = centered_rect(20, 5, area) {
            let v = layout(
                area,
                Direction::Vertical,
                &[Constraint::Length(3), Constraint::Percentage(90)],
            );
            let h = layout(
                v[1],
                Direction::Horizontal,
                &[Constraint::Percentage(50), Constraint::Percentage(50)],
            );

            let (yes, no) = if playlist.yes {
                (underlined(), fg(BrightBlack).dim())
            } else {
                (fg(BrightBlack).dim().underlined(), underlined())
            };

            let delete_msg = if let Mode::Playlist = playlist.mode {
                "Delete playlist?"
            } else {
                "Delete song?"
            };

            buf.clear(area);

            lines!(delete_msg)
                .block(block().borders(Borders::TOP | Borders::LEFT | Borders::RIGHT))
                .align(Center)
                .draw(v[0], buf);

            lines!("Yes".style(yes))
                .block(block().borders(Borders::LEFT | Borders::BOTTOM))
                .align(Center)
                .draw(h[0], buf);

            lines!("No".style(no))
                .block(block().borders(Borders::RIGHT | Borders::BOTTOM))
                .align(Center)
                .draw(h[1], buf);
        }
    } else if let Mode::Popup = playlist.mode {
        //TODO: I think I want a different popup.
        //It should be a small side bar in the browser.
        //There should be a list of existing playlists.
        //The first playlist will be the one you just added to
        //so it's fast to keep adding things
        //The last item will be add a new playlist.
        //If there are no playlists it will prompt you to create on.
        //This should be similar to foobar on android.

        //TODO: Renaming
        //Move items around in lists
        //There should be a hotkey to add to most recent playlist
        //And a message should show up in the bottom bar saying
        //"[name] has been has been added to [playlist name]"
        //or
        //"25 songs have been added to [playlist name]"

        let area = area.centered(45, 6).unwrap();
        buf.clear(area);
        block().draw(area, buf);
        // let v = layout_margin(
        //     area,
        //     Direction::Vertical,
        //     &[Constraint::Length(3), Percentage(50)],
        //     // (1, 1),
        //     (0, 0),
        // );

        //TODO: This doesn't look right.
        // block().title("Add to playlist").margin(1).draw(area, buf);

        // lines!(playlist.search_query.as_str())
        //     .block(block())
        //     .scroll()
        //     .draw(v[0], buf);

        //TODO: Underline `new` and `existing` to clarify what is happening.
        // if playlist.changed {
        //     playlist.changed = false;
        //     let eq = playlist
        //         .lists
        //         .iter()
        //         .any(|p| p.name() == playlist.search_query);
        //     playlist.search_result = if eq {
        //         format!("Add to existing playlist: {}", playlist.search_query)
        //     } else if playlist.search_query.is_empty() {
        //         String::from("Enter a playlist name...")
        //     } else {
        //         format!("Add to new playlist: {}", playlist.search_query)
        //     }
        // }

        // lines!(playlist.search_result.as_str()).draw(v[1].inner((1, 0)), buf);
        // block().draw(v[1], buf);

        //Draw the cursor.
        // let (x, y) = (v[0].x + 1, v[0].y + 1);
        // if playlist.search_query.is_empty() {
        //     return Some((x, y));
        // } else {
        //     let width = v[0].width.saturating_sub(3);
        //     if playlist.search_query.len() < width as usize {
        //         return Some((x + (playlist.search_query.len() as u16), y));
        //     } else {
        //         return Some((x + width, y));
        //     }
        // }
    }
    None
}

pub fn on_enter(playlist: &mut Playlist, songs: &mut Index<Song>) {
    //No was selected by the user.
    if playlist.delete && !playlist.yes {
        playlist.yes = true;
        return playlist.delete = false;
    }

    match playlist.mode {
        Mode::Playlist if playlist.delete => delete_playlist(playlist),
        Mode::Song if playlist.delete => delete_song(playlist),
        Mode::Playlist => {
            if let Some(selected) = playlist.lists.selected() {
                songs.extend(selected.songs.clone());
            }
        }
        Mode::Song => {
            if let Some(selected) = playlist.lists.selected() {
                if let Some(song) = selected.songs.selected() {
                    songs.push(song.clone());
                }
            }
        }
        Mode::Popup if !playlist.song_buffer.is_empty() => {
            //Find the index of the playlist
            let name = playlist.search_query.trim().to_string();
            let pos = playlist.lists.iter().position(|p| p.name() == name);

            let songs = mem::take(&mut playlist.song_buffer);

            //If the playlist exists
            if let Some(pos) = pos {
                let pl = &mut playlist.lists[pos];
                pl.songs.extend(songs);
                pl.songs.select(Some(0));
                pl.save().unwrap();
                playlist.lists.select(Some(pos));
            } else {
                //If the playlist does not exist create it.
                let len = playlist.lists.len();
                playlist.lists.push(gonk_core::Playlist::new(&name, songs));
                playlist.lists[len].save().unwrap();
                playlist.lists.select(Some(len));
            }

            //Reset everything.
            playlist.search_query = String::new();
            playlist.mode = Mode::Playlist;
        }
        Mode::Popup => (),
    }
}

pub fn on_backspace(playlist: &mut Playlist, control: bool) {
    match playlist.mode {
        Mode::Popup => {
            playlist.changed = true;
            if control {
                playlist.search_query.clear();
            } else {
                playlist.search_query.pop();
            }
        }
        _ => left(playlist),
    }
}

pub fn add(playlist: &mut Playlist, songs: Vec<Song>) {
    playlist.song_buffer = songs;
    playlist.mode = Mode::Popup;
}

fn delete_song(playlist: &mut Playlist) {
    if let Some(i) = playlist.lists.index() {
        let selected = &mut playlist.lists[i];

        if let Some(j) = selected.songs.index() {
            selected.songs.remove(j);
            selected.save().unwrap();

            //If there are no songs left delete the playlist.
            if selected.songs.is_empty() {
                selected.delete().unwrap();
                playlist.lists.remove_and_move(i);
                playlist.mode = Mode::Playlist;
            }
        }
        playlist.delete = false;
    }
}

fn delete_playlist(playlist: &mut Playlist) {
    if let Some(index) = playlist.lists.index() {
        playlist.lists[index].delete().unwrap();
        playlist.lists.remove_and_move(index);
        playlist.delete = false;
    }
}

pub fn delete(playlist: &mut Playlist, shift: bool) {
    match playlist.mode {
        Mode::Playlist if shift => delete_playlist(playlist),
        Mode::Song if shift => delete_song(playlist),
        Mode::Playlist | Mode::Song => {
            playlist.delete = true;
        }
        Mode::Popup => (),
    }
}
