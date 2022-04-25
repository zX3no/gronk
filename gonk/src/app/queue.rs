use super::COLORS;
use crate::widget::{Cell, Gauge, Row, Table, TableState};
use crossterm::event::KeyModifiers;
use gonk_core::Index;
use gonk_player::Player;
use tui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use tui::style::{Color, Modifier, Style};
use tui::text::{Span, Spans};
use tui::widgets::{Block, BorderType, Borders, Paragraph};
use tui::{backend::Backend, Frame};

pub struct Queue {
    pub ui: Index<()>,
    pub constraint: [u16; 4],
    pub clicked_pos: Option<(u16, u16)>,
    pub player: Player,
}

impl Queue {
    pub fn new(vol: u16) -> Self {
        Self {
            ui: Index::default(),
            constraint: [8, 42, 24, 26],
            clicked_pos: None,
            player: Player::new(vol),
        }
    }
    pub fn update(&mut self) {
        if self.ui.is_none() && !self.player.songs.is_empty() {
            self.ui.select(Some(0));
        }
        self.player.update();
    }
    pub fn move_constraint(&mut self, arg: char, modifier: KeyModifiers) {
        //1 is 48, '1' - 49 = 0
        let i = (arg as usize) - 49;
        if modifier == KeyModifiers::SHIFT && self.constraint[i] != 0 {
            self.constraint[i] = self.constraint[i].saturating_sub(1);
            self.constraint[i + 1] += 1;
        } else if self.constraint[i + 1] != 0 {
            self.constraint[i] += 1;
            self.constraint[i + 1] = self.constraint[i + 1].saturating_sub(1);
        }

        for n in &mut self.constraint {
            if *n > 100 {
                *n = 100;
            }
        }

        assert!(
            self.constraint.iter().sum::<u16>() == 100,
            "Constraint went out of bounds: {:?}",
            self.constraint
        );
    }
    pub fn up(&mut self) {
        self.ui.up_with_len(self.player.songs.len());
    }
    pub fn down(&mut self) {
        self.ui.down_with_len(self.player.songs.len());
    }

    pub fn clear(&mut self) {
        self.player.clear_songs();
        self.ui = Index::default();
    }
}

impl Queue {
    pub fn draw<B: Backend>(&mut self, f: &mut Frame<B>) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(10),
                Constraint::Length(3),
            ])
            .split(f.size());

        self.draw_header(f, chunks[0]);
        self.draw_body(f, chunks[1]);
        //TODO: if allow for old and new seeker
        self.draw_new_seeker(f, chunks[2]);
    }
    fn draw_header<B: Backend>(&mut self, f: &mut Frame<B>, chunk: Rect) {
        //Render the borders first
        let b = Block::default()
            .borders(Borders::TOP | Borders::LEFT | Borders::RIGHT)
            .border_type(BorderType::Rounded);
        f.render_widget(b, chunk);

        //Left
        let time = if self.player.songs.is_empty() {
            String::from("╭─Stopped")
        } else if !self.player.is_paused() {
            let duration = self.player.duration;
            let elapsed = self.player.elapsed();

            let mins = elapsed / 60.0;
            let rem = elapsed % 60.0;
            let e = format!("{:02}:{:02}", mins.trunc(), rem.trunc());

            let mins = duration / 60.0;
            let rem = duration % 60.0;
            let d = format!("{:02}:{:02}", mins.trunc(), rem.trunc());

            format!("╭─{}/{}", e, d)
        } else {
            String::from("╭─Paused")
        };

        let left = Paragraph::new(time).alignment(Alignment::Left);
        f.render_widget(left, chunk);

        //Center
        if !self.player.songs.is_empty() {
            self.draw_title(f, chunk);
        }

        //Right
        let text = Spans::from(format!("Vol: {}%─╮", self.player.volume));
        let right = Paragraph::new(text).alignment(Alignment::Right);
        f.render_widget(right, chunk);
    }
    fn draw_title<B: Backend>(&mut self, f: &mut Frame<B>, chunk: Rect) {
        let center = if let Some(song) = self.player.songs.selected() {
            let mut artist = song.artist.clone();
            let mut name = song.name.clone();
            let width = chunk.width.saturating_sub(45) as usize;

            while artist.len() + name.len() + "-| - |-".len() > width {
                if artist.len() > name.len() {
                    artist.pop();
                } else {
                    name.pop();
                }
            }

            vec![
                Spans::from(vec![
                    Span::raw("─| "),
                    Span::styled(
                        artist.trim_end().to_string(),
                        Style::default().fg(COLORS.artist),
                    ),
                    Span::raw(" - "),
                    Span::styled(&song.name, Style::default().fg(COLORS.title)),
                    Span::raw(" |─"),
                ]),
                Spans::from(Span::styled(&song.album, Style::default().fg(COLORS.album))),
            ]
        } else {
            vec![Spans::default(), Spans::default()]
        };

        //TODO: scroll the text to the left
        let center = Paragraph::new(center).alignment(Alignment::Center);
        f.render_widget(center, chunk);
    }
    fn draw_body<B: Backend>(&mut self, f: &mut Frame<B>, chunk: Rect) {
        let border = Borders::LEFT | Borders::BOTTOM | Borders::RIGHT;
        if self.player.songs.is_empty() {
            return f.render_widget(
                Block::default()
                    .borders(border)
                    .border_type(BorderType::Rounded),
                chunk,
            );
        }

        let (songs, now_playing, ui_index) = (
            &self.player.songs.data,
            self.player.songs.index,
            self.ui.index,
        );

        let mut items: Vec<Row> = songs
            .iter()
            .map(|song| {
                Row::new(vec![
                    Cell::from(""),
                    Cell::from(song.number.to_string()).style(Style::default().fg(COLORS.track)),
                    Cell::from(song.name.clone()).style(Style::default().fg(COLORS.title)),
                    Cell::from(song.album.clone()).style(Style::default().fg(COLORS.album)),
                    Cell::from(song.artist.clone()).style(Style::default().fg(COLORS.artist)),
                ])
            })
            .collect();

        if let Some(playing_index) = now_playing {
            if let Some(song) = songs.get(playing_index) {
                if let Some(ui_index) = ui_index {
                    //Currently playing song
                    let row = if ui_index == playing_index {
                        Row::new(vec![
                            Cell::from(">>").style(
                                Style::default()
                                    .fg(Color::White)
                                    .add_modifier(Modifier::DIM | Modifier::BOLD),
                            ),
                            Cell::from(song.number.to_string())
                                .style(Style::default().bg(COLORS.track).fg(Color::Black)),
                            Cell::from(song.name.clone())
                                .style(Style::default().bg(COLORS.title).fg(Color::Black)),
                            Cell::from(song.album.clone())
                                .style(Style::default().bg(COLORS.album).fg(Color::Black)),
                            Cell::from(song.artist.clone())
                                .style(Style::default().bg(COLORS.artist).fg(Color::Black)),
                        ])
                    } else {
                        Row::new(vec![
                            Cell::from(">>").style(
                                Style::default()
                                    .fg(Color::White)
                                    .add_modifier(Modifier::DIM | Modifier::BOLD),
                            ),
                            Cell::from(song.number.to_string())
                                .style(Style::default().fg(COLORS.track)),
                            Cell::from(song.name.clone()).style(Style::default().fg(COLORS.title)),
                            Cell::from(song.album.clone()).style(Style::default().fg(COLORS.album)),
                            Cell::from(song.artist.clone())
                                .style(Style::default().fg(COLORS.artist)),
                        ])
                    };

                    items.remove(playing_index);
                    items.insert(playing_index, row);

                    //Current selection
                    if ui_index != playing_index {
                        if let Some(song) = songs.get(ui_index) {
                            let row = Row::new(vec![
                                Cell::from(""),
                                Cell::from(song.number.to_string())
                                    .style(Style::default().bg(COLORS.track)),
                                Cell::from(song.name.clone())
                                    .style(Style::default().bg(COLORS.title)),
                                Cell::from(song.album.clone())
                                    .style(Style::default().bg(COLORS.album)),
                                Cell::from(song.artist.clone())
                                    .style(Style::default().bg(COLORS.artist)),
                            ])
                            .style(Style::default().fg(Color::Black));
                            items.remove(ui_index);
                            items.insert(ui_index, row);
                        }
                    }
                }
            }
        }

        let con = [
            Constraint::Length(2),
            Constraint::Percentage(self.constraint[0]),
            Constraint::Percentage(self.constraint[1]),
            Constraint::Percentage(self.constraint[2]),
            Constraint::Percentage(self.constraint[3]),
        ];

        let t = Table::new(items)
            .header(
                Row::new(vec!["", "Track", "Title", "Album", "Artist"])
                    .style(
                        Style::default()
                            .fg(Color::White)
                            .add_modifier(Modifier::BOLD),
                    )
                    .bottom_margin(1),
            )
            .block(
                Block::default()
                    .borders(border)
                    .border_type(BorderType::Rounded),
            )
            .widths(&con);

        // handle mouse
        if let Some((_, height)) = self.clicked_pos {
            let (start, _) = t.get_row_bounds(ui_index, t.get_row_height(chunk));
            //the header is 5 units long
            if height >= 5 {
                let index = (height - 5) as usize + start;
                //don't select out of bounds
                if index < self.player.songs.len() {
                    self.ui.select(Some(index));
                    self.clicked_pos = None;
                }
            }
        }

        //required to scroll songs
        let mut state = TableState::new(ui_index);
        f.render_stateful_widget(t, chunk, &mut state);
    }
    fn draw_new_seeker<B: Backend>(&mut self, f: &mut Frame<B>, chunk: Rect) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded);

        let elapsed = self.player.elapsed();
        let duration = self.player.duration;

        let seeker = format!(
            "{:02}:{:02}/{:02}:{:02}",
            (elapsed / 60.0).floor(),
            elapsed.trunc() as u32 % 60,
            (duration / 60.0).floor(),
            duration.trunc() as u32 % 60,
        );

        let ratio = self.player.elapsed() / self.player.duration;
        let ratio = if ratio.is_nan() {
            0.0
        } else {
            ratio.clamp(0.0, 1.0)
        };

        let g = Gauge::default()
            .block(block)
            .gauge_style(Style::default().fg(Color::White))
            .ratio(ratio)
            .label(seeker);

        f.render_widget(g, chunk);

        //mouse
        if let Some((width, height)) = self.clicked_pos {
            let size = f.size();
            if size.height - 3 == height || size.height - 2 == height || size.height - 1 == height {
                let ratio = f64::from(width) / f64::from(size.width);
                let duration = self.player.duration;
                let new_time = duration * ratio;
                self.player.seek_to(new_time);
                self.clicked_pos = None;
            }
        }
    }
    #[allow(unused)]
    fn draw_old_seeker<B: Backend>(&self, f: &mut Frame<B>, chunk: Rect) {
        let block = Block::default()
            .borders(Borders::BOTTOM | Borders::LEFT | Borders::RIGHT)
            .border_type(BorderType::Rounded);

        if self.player.songs.is_empty() {
            return f.render_widget(block, chunk);
        }

        let width = f.size().width;
        let ratio = self.player.elapsed() / self.player.duration;
        let pos = (f64::from(width) * ratio) as usize;
        let mut string: String = (0..width.saturating_sub(6))
            .map(|i| if (i as usize) < pos { '=' } else { '-' })
            .collect();

        if pos < string.len().saturating_sub(1) {
            string.remove(pos);
            string.insert(pos, '>');
        } else {
            string.pop();
            string.push('>');
        }

        let p = Paragraph::new(string)
            .alignment(Alignment::Center)
            .block(block);

        f.render_widget(p, chunk);
    }
}
