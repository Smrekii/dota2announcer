use rodio::source::SineWave;
use rodio::{Decoder, OutputStream, Sink, Source};
use std::borrow::Cow;
use std::fs::File;
use std::io::{BufReader, Cursor};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::Mutex;
use std::time::Duration;

pub struct AudioPlayer {
    handle: Mutex<Sender<PlayCommand>>,
}

impl AudioPlayer {
    pub fn new() -> Self {
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(audio_task(rx));
        AudioPlayer {
            handle: Mutex::new(tx),
        }
    }

    fn send(&self, command: PlayCommand) {
        if let Ok(h) = self.handle.lock() {
            h.send(command).unwrap_or(())
        }
    }

    pub fn play_beep(&self, freq: u32, duration_ms: u16) {
        self.send(PlayCommand::Beep(
            freq,
            Duration::from_millis(duration_ms as u64),
        ))
    }

    pub fn play_file(&self, file: File) {
        self.send(PlayCommand::File(file))
    }

    pub fn play_data(&self, data: Cow<'static, [u8]>) {
        self.send(PlayCommand::Data(data))
    }

    /// 1.0 = 100%
    pub fn set_volume(&self, value: f32) {
        self.send(PlayCommand::Volume(value))
    }
}

enum PlayCommand {
    Beep(u32, Duration),
    File(File),
    Data(Cow<'static, [u8]>),
    Volume(f32),
}

impl PlayCommand {
    fn play(self, sink: &Sink) {
        match self {
            Self::Beep(freq, duration) => play_beep(sink, freq, duration),
            Self::File(file) => play_file(sink, file),
            Self::Data(data) => play_data(sink, data),
            Self::Volume(value) => sink.set_volume(value),
        }
    }
}

fn play_beep(sink: &Sink, freq: u32, duration: Duration) {
    sink.append(SineWave::new(freq).take_duration(duration))
}

fn play_file(sink: &Sink, file: File) {
    if let Ok(source) = Decoder::new(BufReader::new(file)) {
        sink.append(source);
    }
}

fn play_data(sink: &Sink, data: Cow<'static, [u8]>) {
    if let Ok(source) = Decoder::new(Cursor::new(data)) {
        sink.append(source);
    }
}

fn audio_task(rx: Receiver<PlayCommand>) -> impl FnOnce() -> () {
    move || {
        let (_stream, stream_handle) = OutputStream::try_default().unwrap();
        let sink = Sink::try_new(&stream_handle).unwrap();
        loop {
            if let Ok(command) = rx.recv() {
                command.play(&sink)
            }
        }
    }
}
