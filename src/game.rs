use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs::File;
use std::io;
use std::io::{BufRead, BufReader};
use std::ops::Deref;
use std::path::{Path, PathBuf};
use winreg::enums::HKEY_LOCAL_MACHINE;
use winreg::RegKey;

pub const GAME_STATE_INTEGRATION_FILE_NAME: &'static str = "gamestate_integration_announcer.cfg";

#[derive(Deserialize, Debug)]
#[serde(default)]
pub struct Game {
    pub provider: Option<Provider>,
    pub map: Option<Map>,
    pub player: Value,
    pub hero: Value,
    pub abilities: Value,
    pub items: Value,
    pub previously: Value,
}

impl Default for Game {
    fn default() -> Self {
        Game {
            provider: None,
            map: None,
            player: Value::Null,
            hero: Value::Null,
            abilities: Value::Null,
            items: Value::Null,
            previously: Value::Null,
        }
    }
}

#[derive(Deserialize, Debug)]
pub struct Provider {
    pub name: String,
    pub appid: i32,
    pub version: i32,
    pub timestamp: u32,
}

#[derive(Deserialize, Debug)]
pub struct Map {
    pub name: String,
    pub matchid: String,
    pub game_time: i32,
    pub clock_time: i32,
    pub daytime: bool,
    pub nightstalker_night: bool,
    pub game_state: String,
    pub paused: bool,
    pub win_team: String,
    pub customgamename: String,
    pub ward_purchase_cooldown: i32,
}
//
// #[derive(Deserialize, Debug)]
// pub struct Player {
//     steamid: String,
//     name: String,
//     activity: String,
//     kills: u32,
//     deaths: u32,
//     assists: u32,
//     last_hits: u32,
//     denies: u32,
//     kill_streak: u32,
//     commands_issued: u32,
//     //kill_list
//     team_name: String,
//     gold: u32,
//     gold_reliable: u32,
//     gold_unreliable: u32,
//     gold_from_hero_kills: u32,
//     gold_from_creep_kills: u32,
//     gold_from_income: u32,
//     gold_from_shared: u32,
//     gpm: u32,
//     xpm: u32
// }
//
// #[derive(Deserialize, Debug)]
// pub struct Hero {
//     xpos: i32,
//     ypos: i32,
//     id: u32,
//     name: String,
//     level: u8,
//     alive: bool,
//     respawn_seconds: u32,
//     buyback_cost: u32,
//     buyback_cooldown: u32,
//     health: u32,
//     max_health: u32,
//     health_percent: u8,
//     mana: u32,
//     max_mana: u32,
//     mana_percent: u8,
//     silenced: bool,
//     stunned: bool,
//     disarmed: bool,
//     magicimmune: bool,
//     hexed: bool,
//     muted: bool,
//     #[serde(rename = "break")]
//     is_break: bool,
//     smoked: bool,
//     has_debuff: bool,
//     talent_1: bool,
//     talent_2: bool,
//     talent_3: bool,
//     talent_4: bool,
//     talent_5: bool,
//     talent_6: bool,
//     talent_7: bool,
//     talent_8: bool
// }

#[derive(Serialize)]
pub struct DotaDir(PathBuf);

impl DotaDir {
    pub fn integration_dir(&self) -> PathBuf {
        self.0.join("game\\dota\\cfg\\gamestate_integration")
    }
}

impl Deref for DotaDir {
    type Target = Path;

    fn deref(&self) -> &Self::Target {
        self.0.as_path()
    }
}

pub fn get_dota2_dir() -> Option<DotaDir> {
    let steam_dir = get_steam_dir().ok()?;
    let steam_apps_dir = steam_dir.join("steamapps");

    for line in read_lines(&steam_apps_dir.join("appmanifest_570.acf")) {
        if let Ok(line) = line {
            if line.contains("installdir") {
                let dir_name = line.replace("\"installdir\"", "").replace("\"", "");

                for library in get_steam_library_folders(&steam_apps_dir) {
                    let dir = library.join(dir_name.trim());
                    if dir.exists() {
                        return Some(DotaDir(dir));
                    }
                }
            }
        }
    }

    None
}

fn get_steam_dir() -> io::Result<PathBuf> {
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let steam_key = hklm
        .open_subkey("SOFTWARE\\Valve\\Steam")
        .or(hklm.open_subkey("SOFTWARE\\Wow6432Node\\Valve\\Steam"))?;

    steam_key
        .get_value("InstallPath")
        .map(|path: String| PathBuf::from(path))
}

fn get_steam_library_folders(steam_apps_dir: &Path) -> Vec<PathBuf> {
    let common_dir = steam_apps_dir.join("common");

    let library_vdf = steam_apps_dir.join("libraryfolders.vdf");
    if !library_vdf.exists() {
        // user has only common library place
        return vec![common_dir];
    }

    let mut folders = Vec::new();
    folders.push(common_dir);

    // we need to analyze vdf file
    let mut index = 1;
    for line in read_lines(&library_vdf) {
        if let Ok(line) = line {
            let lib = format!("\"{}\"", index);
            if !line.contains(&lib) {
                break;
            }

            folders.push(PathBuf::from(
                line.replace(&lib, "").replace("\"", "").trim(),
            ));
            index = index + 1;
        }
    }
    folders
}

fn read_lines(path: &Path) -> Box<dyn Iterator<Item = io::Result<String>>> {
    match File::open(path).map(BufReader::new).map(BufReader::lines) {
        Ok(lines) => Box::new(lines),
        Err(e) => {
            println!("Unable to read lines from file {:?}: {:?}", path, e);
            Box::new(std::iter::empty())
        }
    }
}
