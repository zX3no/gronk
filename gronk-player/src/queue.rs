use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct QueueSong {
    pub path: PathBuf,
    pub elapsed: Option<f64>,
    pub duration: Option<f64>,
}
impl QueueSong {
    pub fn from(path: PathBuf) -> Self {
        Self {
            path,
            elapsed: None,
            duration: None,
        }
    }
    pub fn from_vec(songs: Vec<PathBuf>) -> Vec<Self> {
        songs
            .iter()
            .map(|song| QueueSong::from(song.to_owned()))
            .collect()
    }
    pub fn update(&mut self, elapsed: f64, duration: f64) {
        self.elapsed = Some(elapsed);
        self.duration = Some(duration);
    }
}
impl PartialEq for QueueSong {
    fn eq(&self, other: &Self) -> bool {
        self.path == other.path
    }
}

#[derive(Debug, Clone)]
pub struct Queue {
    pub songs: Vec<QueueSong>,
    pub now_playing: Option<QueueSong>,
    pub index: Option<usize>,
    pub percent: u16,
}
impl Queue {
    pub fn new() -> Self {
        Self {
            songs: Vec::new(),
            now_playing: None,
            index: None,
            percent: 0,
        }
    }
    // pub fn test() -> Self {
    //     Self {
    //         songs: vec![Song::from("music/2.flac"), Song::from("music/3.flac")],
    //         now_playing: None,
    //         index: Some(0),
    //         percent: 0,
    //     }
    // }
    pub fn next_song(&mut self) -> Option<PathBuf> {
        if self.now_playing.is_some() {
            if let Some(index) = &mut self.index {
                if let Some(next_song) = self.songs.get(*index + 1) {
                    *index += 1;
                    return Some(next_song.path.clone());
                } else if let Some(next_song) = self.songs.first() {
                    *index = 0;
                    return Some(next_song.path.clone());
                }
            }
        }
        None
    }
    pub fn prev_song(&mut self) -> Option<PathBuf> {
        let (now_playing, index, queue) = (&mut self.now_playing, &mut self.index, &self.songs);

        if let Some(song) = now_playing {
            if let Some(index) = index {
                if *index != 0 {
                    if let Some(next_song) = queue.get(*index - 1) {
                        *song = next_song.clone();
                        *index -= 1;
                    }
                } else if let Some(next_song) = queue.last() {
                    *song = next_song.clone();
                    *index = queue.len() - 1;
                }
            }
            Some(song.path.clone())
        } else {
            None
        }
    }
}
