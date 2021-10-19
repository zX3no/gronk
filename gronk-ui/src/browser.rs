use gronk_indexer::database::Database;
use tui::widgets::{ListItem, ListState};

#[derive(Debug, Clone)]
pub enum BrowserMode {
    Artist,
    Album,
    Song,
}

#[derive(Debug, Clone)]
pub struct Browser {
    pub mode: BrowserMode,
    pub artist: BrowserList,
    pub album: BrowserList,
    pub song: BrowserList,
    pub database: Database,
}

impl<'a> Browser {
    pub fn new() -> Self {
        let database = Database::new(r"D:\OneDrive\Music");
        let artist = BrowserList::get_artists(&database);
        let album = BrowserList::get_albums_from_artist(&database, artist.first());
        let song = BrowserList::get_songs_from_album(&database, album.first());

        Self {
            mode: BrowserMode::Artist,
            artist,
            album,
            song,
            database,
        }
    }
    //updates the albums or songs depending on what was selected
    pub fn update(&mut self) {
        match self.mode {
            BrowserMode::Album => {
                if let Some(artist) = self.artist.get_name() {
                    self.album = BrowserList::get_albums_from_artist(&self.database, artist);
                }
            }
            BrowserMode::Song => {
                if let Some(album) = self.album.get_name() {
                    self.song = BrowserList::get_songs_from_album(&self.database, album);
                }
            }
            _ => (),
        }
    }

    pub fn get_list(&self) -> Vec<ListItem<'a>> {
        let list = match self.mode {
            BrowserMode::Artist => &self.artist.list,
            BrowserMode::Album => &self.album.list,
            BrowserMode::Song => &self.song.list,
        };

        list.iter().map(|l| ListItem::new(l.clone())).collect()
    }

    fn get_selection(&mut self) -> &mut BrowserList {
        return match self.mode {
            BrowserMode::Artist => &mut self.artist,
            BrowserMode::Album => &mut self.album,
            BrowserMode::Song => &mut self.song,
        };
    }
    pub fn selected(&mut self) -> &mut ListState {
        return match self.mode {
            BrowserMode::Artist => &mut self.artist.selection,
            BrowserMode::Album => &mut self.album.selection,
            BrowserMode::Song => &mut self.song.selection,
        };
    }
    pub fn next_mode(&mut self) {
        match self.mode {
            BrowserMode::Artist => self.mode = BrowserMode::Album,
            BrowserMode::Album => self.mode = BrowserMode::Song,
            BrowserMode::Song => (),
        }
        self.update();
    }
    pub fn prev_mode(&mut self) {
        match self.mode {
            BrowserMode::Artist => (),
            BrowserMode::Album => self.mode = BrowserMode::Artist,
            BrowserMode::Song => self.mode = BrowserMode::Album,
        }
    }
    pub fn title(&self) -> String {
        return match self.mode {
            BrowserMode::Artist => String::from("Artist"),
            BrowserMode::Album => String::from("Album"),
            BrowserMode::Song => String::from("Song"),
        };
    }
    pub fn up(&mut self) {
        let selection = self.get_selection();
        selection.up();
    }
    pub fn down(&mut self) {
        let selection = self.get_selection();
        selection.down();
    }
    pub fn is_song(&self) -> bool {
        if let BrowserMode::Song = self.mode {
            return true;
        }
        return false;
    }
    pub fn filter_album_by_artist() {
        todo!();
    }
    pub fn filter_song_by_album() {
        todo!();
    }
}

//change browser list to three different types
//artist
//album
//song
//they all derive the trait Browser
//the trait is
//up
//down

#[derive(Debug, Clone)]
pub struct BrowserList {
    // list: Vec<ListItem<'a>>,
    list: Vec<String>,
    selection: ListState,
}

impl BrowserList {
    pub fn get_name(&self) -> Option<&String> {
        if let Some(index) = self.selection.selected() {
            return self.list.get(index);
        }
        None
    }
    pub fn first(&self) -> &String {
        self.list.first().unwrap()
    }
    pub fn get_artists(database: &Database) -> Self {
        let mut list = Vec::new();
        for artist in &database.artists {
            list.push(artist.name.clone());
        }

        let mut selection = ListState::default();
        selection.select(Some(0));
        Self { list, selection }
    }
    pub fn get_songs_from_album(database: &Database, name: &String) -> Self {
        let album = database.find_album(&name).unwrap();

        let list: Vec<String> = album
            .songs
            .iter()
            .map(|song| song.name_with_number.clone())
            .collect();
        let mut selection = ListState::default();
        selection.select(Some(0));
        Self { list, selection }
    }
    pub fn get_albums_from_artist(database: &Database, name: &String) -> Self {
        let artist = database.find_artist(&name).unwrap();
        let list: Vec<String> = artist
            .albums
            .iter()
            .map(|album| album.name.clone())
            .collect();

        let mut selection = ListState::default();
        selection.select(Some(0));

        Self { list, selection }
    }
    pub fn down(&mut self) {
        let len = self.list.len();
        let selection = &mut self.selection;
        let selected = selection.selected();

        if let Some(selected) = selected {
            if selected + 1 > len - 1 {
                selection.select(Some(0));
            } else {
                selection.select(Some(selected + 1));
            }
        }
    }
    pub fn up(&mut self) {
        let len = self.list.len();
        let selection = &mut self.selection;
        let selected = selection.selected();

        if let Some(selected) = selected {
            if selected != 0 {
                selection.select(Some(selected - 1));
            } else {
                selection.select(Some(len - 1));
            }
        }
    }
}
