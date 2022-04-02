use crate::{CONFIG_DIR, DB_DIR};
use dpc_pariter::IteratorExt;
use gonk_types::Song;
use jwalk::WalkDir;
use rusqlite::{params, Connection, Params, Row};
use std::{
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
    time::Duration,
};

fn fix(item: &str) -> String {
    item.replace('\'', r"''")
}

pub struct Database {
    conn: Connection,
    busy: Arc<AtomicBool>,
}

impl Database {
    pub fn new() -> rusqlite::Result<Self> {
        if !Path::new(CONFIG_DIR.as_path()).exists() {
            std::fs::create_dir(CONFIG_DIR.as_path()).unwrap();
        }

        if !Path::new(DB_DIR.as_path()).exists() {
            let conn = Connection::open(DB_DIR.as_path()).unwrap();
            conn.busy_timeout(Duration::from_millis(0))?;
            conn.pragma_update(None, "journal_mode", "WAL")?;
            conn.pragma_update(None, "synchronous", "0")?;
            conn.pragma_update(None, "temp_store", "MEMORY")?;

            conn.execute(
                "CREATE TABLE song (
                    number   INTEGER NOT NULL,
                    disc     INTEGER NOT NULL,
                    name     TEXT NOT NULL,
                    album    TEXT NOT NULL,
                    artist   TEXT NOT NULL,
                    path     TEXT NOT NULL UNIQUE,
                    duration DOUBLE NOT NULL,
                    parent   TEXT NOT NULL
                )",
                [],
            )?;
        }

        Ok(Self {
            conn: Connection::open(DB_DIR.as_path()).unwrap(),
            busy: Arc::new(AtomicBool::new(false)),
        })
    }
    pub fn is_busy(&self) -> bool {
        self.busy.load(Ordering::Relaxed)
    }
    pub fn sync_database(&self, toml_paths: &[String]) {
        let mut stmt = self
            .conn
            .prepare("SELECT DISTINCT parent FROM song")
            .unwrap();

        let paths: Vec<_> = stmt
            .query_map([], |row| row.get(0))
            .unwrap()
            .flatten()
            .collect();

        //delete paths that aren't in the toml file but are in the database
        paths.iter().for_each(|path| {
            if !toml_paths.contains(path) {
                self.conn
                    .execute("DELETE FROM song WHERE parent = ?", [path])
                    .unwrap();
            }
        });

        //find the paths that are missing from the database
        let paths_to_add: Vec<_> = toml_paths
            .iter()
            .filter_map(|path| {
                if !paths.contains(path) {
                    Some(path.clone())
                } else {
                    None
                }
            })
            .collect();

        if !paths_to_add.is_empty() {
            self.add_dirs(paths_to_add);
        }
    }
    pub fn add_dirs(&self, dirs: Vec<String>) {
        let busy = self.busy.clone();
        busy.store(true, Ordering::SeqCst);

        thread::spawn(move || {
            for dir in dirs {
                let songs: Vec<Song> = WalkDir::new(&dir)
                    .into_iter()
                    .map(|dir| dir.unwrap().path())
                    .filter(|dir| {
                        if let Some(ex) = dir.extension() {
                            matches!(ex.to_str(), Some("flac" | "mp3" | "ogg" | "wav" | "m4a"))
                        } else {
                            false
                        }
                    })
                    .parallel_map(|dir| Song::from(&dir))
                    .collect();

                if songs.is_empty() {
                    return busy.store(false, Ordering::SeqCst);
                }

                let mut stmt = String::from("BEGIN;\n");
                stmt.push_str(&songs.iter()
                .map(|song| {
                    let artist = fix(&song.artist);
                    let album = fix(&song.album);
                    let name = fix(&song.name);
                    let path = fix(song.path.to_str().unwrap());
                    let parent = fix(&dir);
                    //TODO: would be nice to have batch params, don't think it's implemented.
                    format!("INSERT OR IGNORE INTO song (number, disc, name, album, artist, path, duration, parent) VALUES ('{}', '{}', '{}', '{}', '{}', '{}', '{}', '{}');",
                                song.number, song.disc, name, album, artist,path, song.duration.as_secs_f64(), parent)
                })
                .collect::<Vec<_>>().join("\n"));

                stmt.push_str("COMMIT;\n");

                let conn = Connection::open(DB_DIR.as_path()).unwrap();

                conn.execute_batch(&stmt).unwrap();
            }

            busy.store(false, Ordering::SeqCst);
        });
    }
    pub fn get_songs_from_id(&self, ids: &[usize]) -> Vec<Song> {
        ids.iter()
            .filter_map(|id| {
                self.collect_songs("SELECT * FROM song WHERE rowid = ?", params![id])
                    .first()
                    .cloned()
            })
            .collect()
    }
    pub fn get_all_songs(&self) -> Vec<(usize, Song)> {
        let mut stmt = self.conn.prepare("SELECT *, rowid FROM song").unwrap();

        stmt.query_map([], |row| {
            let id = row.get(8).unwrap();
            let song = Database::song(row);
            Ok((id, song))
        })
        .unwrap()
        .flatten()
        .collect()
    }
    pub fn get_all_artists(&self) -> Vec<String> {
        let mut stmt = self
            .conn
            .prepare("SELECT DISTINCT artist FROM song ORDER BY artist COLLATE NOCASE")
            .unwrap();

        stmt.query_map([], |row| {
            let artist: String = row.get(0).unwrap();
            Ok(artist)
        })
        .unwrap()
        .flatten()
        .collect()
    }
    pub fn get_all_albums(&self) -> Vec<(String, String)> {
        let mut stmt = self
            .conn
            .prepare("SELECT DISTINCT album, artist FROM song ORDER BY artist COLLATE NOCASE")
            .unwrap();

        stmt.query_map([], |row| {
            let album: String = row.get(0).unwrap();
            let artist: String = row.get(1).unwrap();
            Ok((album, artist))
        })
        .unwrap()
        .flatten()
        .collect()
    }
    pub fn get_all_albums_by_artist(&self, artist: &str) -> Vec<String> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT DISTINCT album FROM song WHERE artist = ? ORDER BY album COLLATE NOCASE",
            )
            .unwrap();

        stmt.query_map([artist], |row| row.get(0))
            .unwrap()
            .flatten()
            .collect()
    }
    pub fn get_songs_by_artist(&self, artist: &str) -> Vec<Song> {
        self.collect_songs(
            "SELECT * FROM song WHERE artist = ? ORDER BY album, disc, number",
            params![artist],
        )
    }
    pub fn get_songs_from_album(&self, album: &str, artist: &str) -> Vec<Song> {
        self.collect_songs(
            "SELECT * FROM song WHERE artist=(?1) AND album=(?2) ORDER BY disc, number",
            params![artist, album],
        )
    }
    pub fn get_song(&self, song: &(u16, String), album: &str, artist: &str) -> Vec<Song> {
        self.collect_songs(
            "SELECT * FROM song WHERE name=(?1) AND number=(?2) AND artist=(?3) AND album=(?4)",
            params![song.1, song.0, artist, album],
        )
    }
    fn collect_songs<P>(&self, query: &str, params: P) -> Vec<Song>
    where
        P: Params,
    {
        let mut stmt = self.conn.prepare(query).unwrap();

        stmt.query_map(params, |row| Ok(Database::song(row)))
            .unwrap()
            .flatten()
            .collect()
    }
    fn song(row: &Row) -> Song {
        let path: String = row.get(5).unwrap();
        let dur: f64 = row.get(6).unwrap();
        Song {
            number: row.get(0).unwrap(),
            disc: row.get(1).unwrap(),
            name: row.get(2).unwrap(),
            album: row.get(3).unwrap(),
            artist: row.get(4).unwrap(),
            duration: Duration::from_secs_f64(dur),
            path: PathBuf::from(path),
        }
    }
    pub fn delete() {
        if DB_DIR.as_path().exists() {
            std::fs::remove_file(DB_DIR.as_path()).unwrap();
        }
    }
}
