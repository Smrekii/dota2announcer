// run headless (it will show only tray icon)
#![windows_subsystem = "windows"]

mod audio;
mod embed;
mod game;
mod settings;

#[macro_use]
extern crate rocket;
#[macro_use]
extern crate rocket_contrib;

use rocket_contrib::json::{Json, JsonValue};
use rocket_contrib::serve::Options;

use crate::audio::AudioPlayer;
use crate::embed::{EmbedFile, EmbedFiles};
use crate::game::{get_dota2_dir, DotaDir, Game, GAME_STATE_INTEGRATION_FILE_NAME};
use crate::settings::{NotifyAction, OnClock, Settings};
use rocket::fairing::AdHoc;
use rocket::figment::Figment;
use rocket::http::{Header, Status};
use rocket::logger::LogLevel;
use rocket::response::status::Custom;
use rocket::response::{Debug, Redirect};
use rocket::{Config, State};
use rust_embed::RustEmbed;
use serde_json::Value;
use std::fs::File;
use std::io;
use std::io::Write;
use std::ops::{Deref, DerefMut};
use std::sync::Mutex;
use std::thread::spawn;
use systray::Application;

const VERSION: &'static str = env!("CARGO_PKG_VERSION");

#[derive(RustEmbed, Clone)]
#[folder = "$CARGO_MANIFEST_DIR/web"]
struct Asset;

struct Runtime {
    player: AudioPlayer,
    settings: Mutex<Settings>,
    last_buyback: Mutex<bool>,
}

#[post("/", format = "json", data = "<state>")]
fn game_state_update(r: State<Runtime>, state: Json<Game>) {
    if let Ok(s) = r.settings.lock() {
        if s.global.suspend_all {
            return;
        }

        if let Some(map) = &state.map {
            if let Some(Value::String(prev_state)) = state.previously.pointer("/map/game_state") {
                println!("{} -> {}", prev_state, map.game_state);
            }

            if map.game_state == "DOTA_GAMERULES_STATE_GAME_IN_PROGRESS"
                || map.game_state == "DOTA_GAMERULES_STATE_PRE_GAME"
            {
                if s.buyback_ready.notify.enabled
                    && (state.previously.pointer("/player/gold_reliable").is_some()
                        || state.previously.pointer("/hero/buyback_cost").is_some()
                        || state.previously.pointer("/hero/buyback_cooldown").is_some())
                {
                    let gold_reliable = state.player["gold_reliable"].as_i64();
                    let buyback_cost = state.hero["buyback_cost"].as_i64();
                    let buyback_cooldown = state.hero["buyback_cooldown"].as_i64();

                    let has_enough_gold = match (gold_reliable, buyback_cost) {
                        (Some(gold), Some(cost)) => gold - cost > 0,
                        _ => false,
                    };

                    let has_buyback = has_enough_gold && buyback_cooldown.unwrap_or_default() == 0;
                    if let Ok(mut last_buyback) = r.last_buyback.lock() {
                        if has_buyback && !*last_buyback.deref() {
                            *last_buyback.deref_mut() = true;
                            s.buyback_ready.notify.action.trigger(&r.player);
                        } else if !has_buyback && *last_buyback.deref() {
                            *last_buyback.deref_mut() = false;
                        }
                    }
                }

                if s.observer_wards.notify.enabled
                    && map.ward_purchase_cooldown == s.observer_wards.notify.before_sec as i32
                    && state
                        .previously
                        .pointer("/map/ward_purchase_cooldown")
                        .is_some()
                {
                    println!(
                        "{} there are observer wards about to spawn in {} sec",
                        map.clock_time, s.observer_wards.notify.before_sec
                    );
                    s.observer_wards.notify.action.trigger(&r.player);
                }

                // handle OnClock actions
                if !state.previously.pointer("/map/clock_time").is_some() {
                    return;
                }

                if s.bounty_rune.on_clock(map.clock_time, &r.player) {
                    println!(
                        "{} there are bounty runes about to spawn in {} sec",
                        map.clock_time, s.bounty_rune.notify.before_sec
                    );
                }

                if s.power_rune.on_clock(map.clock_time, &r.player) {
                    println!(
                        "{} there are power runes about to spawn in {} sec",
                        map.clock_time, s.power_rune.notify.before_sec
                    );
                }

                if s.tomb_of_knowledge.on_clock(map.clock_time, &r.player) {
                    println!(
                        "{} there is tomb of knowledge about to spawn in {} sec",
                        map.clock_time, s.tomb_of_knowledge.notify.before_sec
                    );
                }

                if s.neutral_items.on_clock(map.clock_time, &r.player) {
                    println!(
                        "{} there are neutral items that can be dropped in about {} sec",
                        map.clock_time, s.neutral_items.notify.before_sec
                    );
                }
            }
        }
    }
}

#[get("/")]
async fn index() -> Redirect {
    Redirect::to("index.html")
}

#[get("/settings")]
fn settings_load(r: State<Runtime>) -> Result<Json<Settings>, ()> {
    match r.settings.lock() {
        Ok(settings) => Ok(Json(settings.clone())),
        Err(_) => Err(()),
    }
}

#[post("/settings", format = "json", data = "<settings>")]
fn settings_save(r: State<Runtime>, settings: Json<Settings>) -> Result<(), Debug<io::Error>> {
    r.settings
        .lock()
        .map_err(|_| io::Error::new(io::ErrorKind::Other, "Lock failed"))
        .and_then(|mut s| {
            *s = settings.into_inner();

            // apply volume change if any
            r.player.set_volume(s.global.volume);

            s.save()
        })
        .map_err(Debug)
}

#[post("/trigger", format = "json", data = "<action>")]
fn trigger(r: State<Runtime>, action: Json<NotifyAction>) {
    action.trigger(&r.player);
}

#[get("/install")]
fn install() -> JsonValue {
    let dota_dir = get_dota2_dir();

    let dota_gamestate_integration_dir = dota_dir.as_ref().map(DotaDir::integration_dir);
    let dota_announcer_integration_file = dota_gamestate_integration_dir
        .as_ref()
        .map(|d| d.join(GAME_STATE_INTEGRATION_FILE_NAME));
    let dota_announcer_integration_file_exists = dota_announcer_integration_file
        .as_ref()
        .map(|f| f.exists())
        .unwrap_or(false);

    json!({
        "dota_dir": dota_dir,
        "dota_gamestate_integration_dir": dota_gamestate_integration_dir,
        "dota_announcer_integration_file": dota_announcer_integration_file,
        "dota_announcer_integration_file_exists": dota_announcer_integration_file_exists,
        "announcer_integration_file_name": GAME_STATE_INTEGRATION_FILE_NAME,
    })
}

#[post("/install")]
fn install_post() -> Result<(), Custom<String>> {
    let integration_file = Asset::get("gamestate_integration_announcer.cfg");

    if let Some(data) = integration_file {
        if let Some(dir) = get_dota2_dir() {
            return File::create(dir.integration_dir().join(GAME_STATE_INTEGRATION_FILE_NAME))
                .and_then(|mut f| f.write_all(&data))
                .map_err(|e| Custom(Status::InternalServerError, e.to_string()));
        }
    }

    Err(Custom(
        Status::InternalServerError,
        "Could not detect Dota2 installation directory".to_string(),
    ))
}

#[get("/gamestate_integration_announcer.cfg")]
async fn integration_file() -> Option<EmbedFile<Asset>> {
    <EmbedFile<Asset>>::of_attachment("gamestate_integration_announcer.cfg").ok()
}

fn spawn_tray_icon(config: Config) {
    spawn(move || {
        if let Ok(mut app) = systray::Application::new() {
            if let Ok(_) = build_tray_menu(&mut app, config) {
                app.wait_for_message()
                    .expect("Failed to handle tray menu messages");
            }
        }
    });
}

fn build_tray_menu(app: &mut Application, config: Config) -> Result<(), systray::Error> {
    app.set_icon_from_resource("icon")?; // see build.rs for resource icon_id
    app.set_tooltip("Dota2 Announcer (by Smreki)")?;

    app.add_menu_item("Config Webpage", move |_| {
        opener::open(format!("http://{}:{}", config.address, config.port))
    })?;

    app.add_menu_separator()?;

    app.add_menu_item("Exit", |window| -> Result<(), systray::Error> {
        window.quit();
        std::process::exit(0);
    })?;

    Ok(())
}

#[launch]
fn rocket() -> rocket::Rocket {
    let figment = Figment::from(Config::release_default()).merge(("log_level", LogLevel::Off));

    // load setting
    let settings = Settings::load();

    // create audio player and set volume form saved settings
    let player = AudioPlayer::new();
    player.set_volume(settings.global.volume);

    let runtime = Runtime {
        player,
        settings: Mutex::new(settings),
        last_buyback: Mutex::new(false),
    };

    rocket::custom(figment)
        .mount("/", <EmbedFiles<Asset>>::new(Options::None))
        .mount("/", routes![index, game_state_update, integration_file])
        .mount(
            "/api",
            routes![settings_load, settings_save, trigger, install, install_post],
        )
        .manage(runtime)
        .attach(AdHoc::on_response("Version header", |_, res| {
            Box::pin(async move {
                res.set_header(Header::new("x-version", VERSION));
            })
        }))
        .attach(AdHoc::on_launch("Launch logger", |rocket| {
            println!(
                "v{} Running at http://{}:{}",
                VERSION,
                rocket.config().address,
                rocket.config().port
            );
            spawn_tray_icon(rocket.config().clone());
        }))
}
