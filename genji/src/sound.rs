//! Genji's sounds and voice (step 11b, DESIGN.md 5j): plays the files tools\make_sounds.py put in
//! D:\GenjiBridgeAssets\sound\<event>\ (Overwatch's own sounds and your Practice Range recordings, outside the
//! repository), through Windows' default output with the rodio library, on a thread of its own. Elden Ring's own
//! audio isn't touched.
//!
//! - Each event picks one of its files at random, never the same one twice in a row (several takes, like Overwatch).
//! - Named channels (Deflect's hum, a voice line) can be stopped or replaced; other sounds just play.
//! - Voice lines don't overlap: a new line waits for none, it is skipped while another is playing (unless urgent).
//! - Volume: Elden Ring's sound effects setting (0-10) times MASTER.
//! - A missing event or file is logged once and skipped.

use std::collections::HashMap;
use std::io::Cursor;
use std::path::Path;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink, Source};

use crate::glog;

const SOUND_DIR: &str = r"D:\GenjiBridgeAssets\sound";
/// Our level on top of Elden Ring's sound effects setting (Overwatch's files are mastered loud).
const MASTER: f32 = 0.8;
/// Per-event level on top of MASTER, from your play test (2026-10-09). Measured from the files: the throw is about
/// 10 dB louder than the hit sounds; "Ryujin no ken wo kurae!" peaks 12 dB under the roar it plays with; the
/// elimination sound peaks at full scale (you: "kinda loud"). Your footstep recording peaks about 28 dB under your dash
/// recording (Overwatch's own mix), raised a little to be heard over Elden Ring. (The critical-health sound was at 0.6
/// and wasn't heard: now at full level.)
const EVENT_GAIN: [(&str, f32); 4] = [("shuriken_throw", 0.45), ("voice_ult_activate", 2.5), ("kill", 0.5), ("footstep", 2.5)];
/// Events not logged each time they play (too frequent).
const QUIET: [&str; 3] = ["footstep", "shuriken_throw", "wall_climb_loop"];

enum Cmd {
    Play { event: &'static str, gain: f32, delay: f32, channel: Option<&'static str> },
    Voice { event: &'static str, urgent: bool },
    Stop(&'static str),
    Volume(f32),
}

static TX: OnceLock<Mutex<Sender<Cmd>>> = OnceLock::new();

fn send(cmd: Cmd) {
    let tx = TX.get_or_init(|| {
        let (tx, rx) = channel();
        std::thread::Builder::new()
            .name("genji-sound".into())
            .spawn(move || run(rx))
            .expect("sound thread");
        Mutex::new(tx)
    });
    let _ = tx.lock().unwrap_or_else(|e| e.into_inner()).send(cmd);
}

/// Plays one take of `event`.
pub fn play(event: &'static str) {
    send(Cmd::Play { event, gain: 1.0, delay: 0.0, channel: None });
}

/// Plays one take of `event` at `gain` (0-1, e.g. quieter with distance).
pub fn play_gain(event: &'static str, gain: f32) {
    send(Cmd::Play { event, gain, delay: 0.0, channel: None });
}

/// Plays one take of `event` after `delay` seconds.
pub fn play_after(event: &'static str, delay: f32) {
    send(Cmd::Play { event, gain: 1.0, delay, channel: None });
}

/// Plays one take of `event` on `channel`, replacing what that channel was playing (`stop` ends it).
pub fn play_on(channel: &'static str, event: &'static str) {
    send(Cmd::Play { event, gain: 1.0, delay: 0.0, channel: Some(channel) });
}

/// `play_on` after `delay` seconds (the channel can be stopped before it starts).
pub fn play_on_after(channel: &'static str, event: &'static str, delay: f32) {
    send(Cmd::Play { event, gain: 1.0, delay, channel: Some(channel) });
}

pub fn stop(channel: &'static str) {
    send(Cmd::Stop(channel));
}

/// A voice line: skipped while another is playing, unless `urgent` (then it cuts in).
pub fn voice(event: &'static str, urgent: bool) {
    send(Cmd::Voice { event, urgent });
}

/// Elden Ring's sound effects setting, 0-10 (sent by the game thread now and then).
pub fn set_game_volume(setting: u8) {
    send(Cmd::Volume(setting as f32 / 10.0));
}

struct Player {
    _stream: OutputStream,
    handle: OutputStreamHandle,
    /// Event -> its files' bytes (read on first use).
    takes: HashMap<&'static str, Vec<Arc<[u8]>>>,
    last_take: HashMap<&'static str, usize>,
    missing_logged: Vec<&'static str>,
    channels: HashMap<&'static str, Sink>,
    volume: f32,
    rng: u64,
}

fn run(rx: Receiver<Cmd>) {
    let (stream, handle) = match OutputStream::try_default() {
        Ok(s) => s,
        Err(e) => {
            glog!("SOUND", "no audio output ({e}); Genji's sounds are off this session");
            return;
        }
    };
    glog!("SOUND", "audio output ready; sounds from {SOUND_DIR}");
    let mut p = Player {
        _stream: stream,
        handle,
        takes: HashMap::new(),
        last_take: HashMap::new(),
        missing_logged: Vec::new(),
        channels: HashMap::new(),
        volume: 1.0,
        rng: 0x9E37_79B9_7F4A_7C15,
    };
    while let Ok(cmd) = rx.recv() {
        match cmd {
            Cmd::Play { event, gain, delay, channel } => p.play(event, gain, delay, channel),
            Cmd::Voice { event, urgent } => {
                let busy = p.channels.get("voice").is_some_and(|s| !s.empty());
                if busy && !urgent {
                    continue;
                }
                p.play(event, 1.0, 0.0, Some("voice"));
            }
            Cmd::Stop(channel) => {
                if let Some(sink) = p.channels.remove(channel) {
                    sink.stop();
                }
            }
            Cmd::Volume(v) => p.volume = v,
        }
    }
}

impl Player {
    fn load(&mut self, event: &'static str) -> &[Arc<[u8]>] {
        self.takes.entry(event).or_insert_with(|| {
            let dir = Path::new(SOUND_DIR).join(event);
            let mut files: Vec<_> = std::fs::read_dir(&dir)
                .map(|it| {
                    it.filter_map(|e| e.ok().map(|e| e.path()))
                        .filter(|p| matches!(p.extension().and_then(|x| x.to_str()), Some("ogg" | "wav")))
                        .collect()
                })
                .unwrap_or_default();
            files.sort();
            files.iter().filter_map(|f| std::fs::read(f).ok().map(Arc::from)).collect()
        })
    }

    fn next_random(&mut self, n: usize) -> usize {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 7;
        self.rng ^= self.rng << 17;
        (self.rng % n as u64) as usize
    }

    fn play(&mut self, event: &'static str, gain: f32, delay: f32, channel: Option<&'static str>) {
        let count = self.load(event).len();
        if count == 0 {
            if !self.missing_logged.contains(&event) {
                glog!("SOUND", "no files for \"{event}\" (run tools\\make_sounds.py?); skipped");
                self.missing_logged.push(event);
            }
            return;
        }
        let mut take = self.next_random(count);
        if count > 1 && self.last_take.get(event) == Some(&take) {
            take = (take + 1) % count;
        }
        self.last_take.insert(event, take);
        let bytes = self.takes[event][take].clone();
        let source = match Decoder::new(Cursor::new(bytes)) {
            Ok(s) => s,
            Err(e) => {
                glog!("SOUND", "\"{event}\" take {}: can't decode ({e})", take + 1);
                return;
            }
        };
        let event_gain = EVENT_GAIN.iter().find(|(e, _)| *e == event).map_or(1.0, |(_, g)| *g);
        let level = (MASTER * self.volume * gain * event_gain).clamp(0.0, 3.0);
        let source = source.convert_samples::<f32>().amplify(level).delay(Duration::from_secs_f32(delay.max(0.0)));
        if !QUIET.contains(&event) {
            glog!("SOUND", "{event} (take {} of {count}){}", take + 1, channel.map(|c| format!(" on {c}")).unwrap_or_default());
        }
        match channel {
            Some(name) => {
                if let Some(old) = self.channels.remove(name) {
                    old.stop();
                }
                if let Ok(sink) = Sink::try_new(&self.handle) {
                    sink.append(source);
                    self.channels.insert(name, sink);
                }
            }
            None => {
                let _ = self.handle.play_raw(source);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every file tools\make_sounds.py put in place decodes with the decoder the mod uses, and isn't silent or empty.
    /// Skipped where the sound folder doesn't exist (it's outside the repository).
    #[test]
    fn every_sound_file_decodes() {
        let Ok(events) = std::fs::read_dir(SOUND_DIR) else {
            return;
        };
        let mut checked = 0;
        for event in events.flatten().filter(|e| e.path().is_dir()) {
            for file in std::fs::read_dir(event.path()).unwrap().flatten() {
                let path = file.path();
                if !matches!(path.extension().and_then(|x| x.to_str()), Some("ogg" | "wav")) {
                    continue;
                }
                let bytes = std::fs::read(&path).unwrap();
                let decoder = Decoder::new(Cursor::new(bytes)).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
                let rate = decoder.sample_rate();
                let samples: Vec<f32> = decoder.convert_samples().collect();
                assert!(rate > 0 && samples.len() > 100, "{}: empty", path.display());
                assert!(samples.iter().any(|s| s.abs() > 0.001), "{}: silent", path.display());
                checked += 1;
            }
        }
        assert!(checked > 300, "only {checked} files");
    }
}
