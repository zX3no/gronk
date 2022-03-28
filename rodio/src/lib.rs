#![cfg_attr(test, deny(missing_docs))]
use cpal::traits::HostTrait;
pub use cpal::{
    self, traits::DeviceTrait, Device, Devices, DevicesError, InputDevices, OutputDevices,
    SupportedStreamConfig,
};
use decoder::Decoder;
use gonk_types::Song;
use rand::prelude::SliceRandom;
use rand::thread_rng;

mod conversions;
mod sink;
mod stream;

pub mod buffer;
pub mod decoder;
pub mod dynamic_mixer;
pub mod queue;
pub mod source;

pub use crate::conversions::Sample;
pub use crate::sink::Sink;
pub use crate::source::Source;
pub use crate::stream::{OutputStream, OutputStreamHandle, PlayError, StreamError};

use std::fs::File;
use std::time::Duration;

static VOLUME_STEP: u16 = 5;

pub struct Player {
    stream: OutputStream,
    handle: OutputStreamHandle,
    sink: Sink,
    total_duration: Option<Duration>,
    pub volume: u16,
    safe_guard: bool,
    pub songs: Vec<Song>,
    pub current_song: Option<usize>,
}

impl Player {
    pub fn new(volume: u16) -> Self {
        let (stream, handle) = OutputStream::try_default().unwrap();
        let sink = Sink::try_new(&handle).unwrap();
        sink.set_volume(volume as f32 / 1000.0);

        Self {
            stream,
            handle,
            sink,
            total_duration: None,
            volume,
            safe_guard: true,
            songs: Vec::new(),
            current_song: None,
        }
    }
    pub fn add_songs(&mut self, song: Vec<Song>) {
        self.songs.extend(song);
        if self.current_song.is_none() && !self.songs.is_empty() {
            self.current_song = Some(0);
            self.play_selected();
        }
    }
    pub fn play_song(&mut self, i: usize) {
        if self.songs.get(i).is_some() {
            self.current_song = Some(i);
            self.play_selected();
        };
    }
    pub fn clear_songs(&mut self) {
        self.songs = Vec::new();
        self.current_song = None;
        self.stop();
    }
    pub fn prev_song(&mut self) {
        if let Some(i) = &self.current_song {
            let i = i.saturating_sub(1);
            if self.songs.get(i).is_some() {
                self.current_song = Some(i);
                self.play_selected();
            };
        }
    }
    pub fn next_song(&mut self) {
        if let Some(i) = &self.current_song {
            let i = i.saturating_add(1);
            if self.songs.get(i).is_some() {
                self.current_song = Some(i);
                self.play_selected();
            };
        }
    }
    pub fn volume_up(&mut self) -> u16 {
        self.volume += VOLUME_STEP;

        if self.volume > 100 {
            self.volume = 100;
        }

        self.sink.set_volume(self.volume as f32 / 1000.0);

        self.volume
    }

    pub fn volume_down(&mut self) -> u16 {
        if self.volume != 0 {
            self.volume -= VOLUME_STEP;
        }

        self.sink.set_volume(self.volume as f32 / 1000.0);

        self.volume
    }
    pub fn sleep_until_end(&self) {
        self.sink.sleep_until_end();
    }
    pub fn play_selected(&mut self) {
        if let Some(i) = self.current_song {
            if let Some(song) = self.songs.get(i).cloned() {
                self.stop();
                let file = File::open(&song.path).unwrap();
                let decoder = Decoder::new(file).unwrap();
                self.total_duration = decoder.total_duration();
                self.sink.append(decoder);
            }
        }
    }
    pub fn delete_song(&mut self, selected: usize) {
        //delete the song from the queue
        self.songs.remove(selected);

        if let Some(current_song) = self.current_song {
            let len = self.songs.len();

            if len == 0 {
                self.clear_songs();
                return;
            } else if current_song == selected && selected == 0 {
                self.current_song = Some(0);
            } else if current_song == selected && len == selected {
                self.current_song = Some(len - 1);
            } else if selected < current_song {
                self.current_song = Some(current_song - 1);
            }

            let end = len.saturating_sub(1);

            if selected > end {
                self.current_song = Some(end);
            }

            //if the playing song was deleted
            //play the next track
            if selected == current_song {
                self.play_selected();
            }
        };
    }
    pub fn randomize(&mut self) {
        if let Some(i) = self.current_song {
            if let Some(song) = self.songs.get(i).cloned() {
                self.songs.shuffle(&mut thread_rng());

                let mut index = 0;
                for (i, s) in self.songs.iter().enumerate() {
                    if s == &song {
                        index = i;
                    }
                }
                self.current_song = Some(index);
            }
        }
    }
    pub fn stop(&mut self) {
        self.sink.destroy();
        self.sink = Sink::try_new(&self.handle).unwrap();
        self.sink.set_volume(self.volume as f32 / 1000.0);
    }
    pub fn elapsed(&self) -> Duration {
        self.sink.elapsed()
    }
    pub fn duration(&self) -> Option<f64> {
        //TODO: wtf is this?
        self.total_duration
            .map(|duration| duration.as_secs_f64() - 0.29)
    }
    pub fn toggle_playback(&self) {
        self.sink.toggle_playback();
    }
    pub fn is_paused(&self) -> bool {
        self.sink.is_paused()
    }
    pub fn seek_fw(&mut self) {
        let seek = self.elapsed().as_secs_f64() + 10.0;
        if let Some(duration) = self.duration() {
            if seek > duration {
                self.safe_guard = true;
            } else {
                self.seek_to(Duration::from_secs_f64(seek));
            }
        }
    }
    pub fn seek_bw(&mut self) {
        let mut seek = self.elapsed().as_secs_f64() - 10.0;
        if seek < 0.0 {
            seek = 0.0;
        }

        self.seek_to(Duration::from_secs_f64(seek));
    }
    pub fn seek_to(&self, time: Duration) {
        self.sink.seek(time);
    }
    pub fn seeker(&self) -> f64 {
        if let Some(duration) = self.duration() {
            let elapsed = self.elapsed();
            elapsed.as_secs_f64() / duration
        } else {
            0.0
        }
    }
    pub fn update(&mut self) {
        if let Some(duration) = self.duration() {
            if self.elapsed().as_secs_f64() > duration {
                self.next_song();
            }
        }
    }
    // pub fn trigger_next(&mut self) -> bool {
    //     if let Some(duration) = self.duration() {
    //         if self.elapsed().as_secs_f64() > duration {
    //             self.safe_guard = true;
    //         }
    //     }

    //     if self.safe_guard {
    //         self.safe_guard = false;
    //         true
    //     } else {
    //         false
    //     }
    // }
    pub fn output_devices() -> Vec<Device> {
        let host_id = cpal::default_host().id();
        let host = cpal::host_from_id(host_id).unwrap();

        let mut devices: Vec<_> = host.output_devices().unwrap().collect();
        devices.sort_by_key(|a| a.name().unwrap().to_lowercase());
        devices
    }
    pub fn default_device() -> Option<Device> {
        cpal::default_host().default_output_device()
    }
    pub fn change_output_device(&mut self, device: &Device) {
        self.stop();
        let (stream, handle) = OutputStream::try_from_device(device).unwrap();
        self.stream = stream;
        self.handle = handle;
    }
}
