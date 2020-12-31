use crate::audio::AudioPlayer;
use crate::Asset;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use std::{fs, io};

/// File to load & store settings from
const SETTINGS_FILE_NAME: &'static str = "settings.json";

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(default)]
pub struct Settings {
    pub global: GlobalConfig,
    pub bounty_rune: SpawnConfig,
    pub power_rune: SpawnConfig,
    pub tomb_of_knowledge: SpawnConfig,
    pub observer_wards: NotifyConfig,
    pub neutral_items: SpawnConfig,
    pub buyback_ready: NotifyConfig,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GlobalConfig {
    pub volume: f32,
    pub suspend_all: bool,
}

impl Settings {
    pub fn load() -> Self {
        let mut serialized = String::new();
        std::env::current_dir()
            .and_then(|cwd| File::open(cwd.join(SETTINGS_FILE_NAME)))
            .and_then(|mut f| f.read_to_string(&mut serialized))
            .and_then(|_| {
                serde_json::from_str(&serialized)
                    .map_err(|err| io::Error::new(io::ErrorKind::Other, err))
            })
            .unwrap_or(Settings::default())
    }

    pub fn save(&self) -> io::Result<()> {
        // serialize our setting to string
        let serialized = serde_json::to_string_pretty(self)?;

        // write them to stage file first
        let cwd = std::env::current_dir()?;
        let json_file_name = cwd.join(SETTINGS_FILE_NAME);
        let stage_file_name = json_file_name.with_extension(".stage");
        File::create(&stage_file_name)?.write_all(serialized.as_bytes())?;

        // once successful rename it
        fs::rename(stage_file_name, json_file_name)
    }
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            global: GlobalConfig {
                volume: 1.0,
                suspend_all: false,
            },
            bounty_rune: SpawnConfig {
                notify: NotifyInfo {
                    enabled: false,
                    before_sec: 15,
                    action: NotifyAction::Sound {
                        sound: "bounty_rune.mp3".to_string(),
                    },
                },
                spawn: SpawnInfo {
                    first_sec: 0,
                    interval_sec: 300,
                },
            },

            power_rune: SpawnConfig {
                notify: NotifyInfo {
                    enabled: false,
                    before_sec: 10,
                    action: NotifyAction::Sound {
                        sound: "power_rune.mp3".to_string(),
                    },
                },
                spawn: SpawnInfo {
                    first_sec: 240,
                    interval_sec: 120,
                },
            },

            tomb_of_knowledge: SpawnConfig {
                notify: NotifyInfo {
                    enabled: false,
                    before_sec: 5,
                    action: NotifyAction::Sound {
                        sound: "tomb_of_knowledge.mp3".to_string(),
                    },
                },
                spawn: SpawnInfo {
                    first_sec: 600,
                    interval_sec: 600,
                },
            },

            observer_wards: NotifyConfig {
                notify: NotifyInfo {
                    enabled: false,
                    before_sec: 0,
                    action: NotifyAction::Sound {
                        sound: "observer_ward.mp3".to_string(),
                    },
                },
            },

            neutral_items: SpawnConfig {
                notify: NotifyInfo {
                    enabled: false,
                    before_sec: 0,
                    action: NotifyAction::Sound {
                        sound: "neutral_items.mp3".to_string(),
                    },
                },
                spawn: SpawnInfo {
                    first_sec: 420,
                    interval_sec: 600,
                },
            },

            buyback_ready: NotifyConfig {
                notify: NotifyInfo {
                    enabled: false,
                    before_sec: 0,
                    action: NotifyAction::Sound {
                        sound: "buyback_ready.mp3".to_string(),
                    },
                },
            },
        }
    }
}

pub trait OnClock {
    fn on_clock(&self, clock_time: i32, player: &AudioPlayer) -> bool;
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SpawnConfig {
    pub notify: NotifyInfo,
    pub spawn: SpawnInfo,
}

impl SpawnConfig {
    fn can_invoke_action(&self, clock_time: i32) -> bool {
        if !self.notify.enabled || clock_time < self.spawn.first_sec as i32 {
            return false;
        }

        let interval_reminder = clock_time % self.spawn.interval_sec as i32;
        let action_reminder = (self.spawn.interval_sec - self.notify.before_sec) as i32;
        interval_reminder == action_reminder
    }
}

impl OnClock for SpawnConfig {
    fn on_clock(&self, clock_time: i32, player: &AudioPlayer) -> bool {
        if self.can_invoke_action(clock_time) {
            self.notify.action.trigger(player);
            return true;
        }
        false
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct NotifyConfig {
    pub notify: NotifyInfo,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SpawnInfo {
    /// Clock time sec when the it is first spawned
    pub first_sec: u16,

    /// Interval in clock time sec when next spawn will occure
    pub interval_sec: u16,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct NotifyInfo {
    pub enabled: bool,
    pub before_sec: u16,
    pub action: NotifyAction,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "lowercase", tag = "type")]
pub enum NotifyAction {
    Beep {
        duration_ms: u16,
        freq: u32,
    },
    Sound {
        // generated from https://ttsmp3.com/
        // US English/Salli
        // stored in web/sound
        sound: String,
    },
    PlayFile {
        path: String,
    },
}

impl NotifyAction {
    fn open_sound(&self, sound: &String) -> Option<Cow<'static, [u8]>> {
        let sound = Path::new("sound").join(sound);
        Asset::get(&sound.to_string_lossy())
    }

    pub fn trigger(&self, player: &AudioPlayer) {
        match &self {
            NotifyAction::Beep { duration_ms, freq } => player.play_beep(*freq, *duration_ms),
            NotifyAction::Sound { sound } => self
                .open_sound(sound)
                .map_or((), |data| player.play_data(data)),
            NotifyAction::PlayFile { path } => {
                File::open(Path::new(path)).map_or((), |file| player.play_file(file))
            }
        }
    }
}

impl Default for NotifyAction {
    fn default() -> Self {
        NotifyAction::Beep {
            duration_ms: 100,
            freq: 400,
        }
    }
}
