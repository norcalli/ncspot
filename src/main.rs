use std::env;
use std::fs::OpenOptions;
use std::io::prelude::*;
use std::path::PathBuf;
use std::process;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::sync::Mutex;

use log::trace;

use cursive::align;
use cursive::direction;
use cursive::event::Key;
use cursive::traits::Identifiable;
use cursive::view::Boxable;
use cursive::view::ScrollStrategy;
use cursive::views::{self, *};
use cursive::Cursive;

use librespot::core::spotify_id::SpotifyId;

use rspotify::spotify::model::track::FullTrack;

mod config;
mod events;
mod queue;
mod spotify;
mod theme;
mod ui;

use crate::events::{Event, EventManager};

fn init_logger(content: TextContent) {
    let mut builder = env_logger::Builder::from_default_env();
    {
        builder
            .format(move |_, record| {
                let mut buffer = content.clone();
                let line = format!("[{}] {}\n", record.level(), record.args());
                buffer.append(line.clone());

                let mut file = OpenOptions::new()
                    .create(true)
                    .write(true)
                    .append(true)
                    .open("ncspot.log")
                    .unwrap();
                if let Err(e) = writeln!(file, "{}", line) {
                    eprintln!("Couldn't write to file: {}", e);
                }
                Ok(())
            })
            .init();
    }
}

fn main() {
    let logbuf = TextContent::new("Welcome to ncspot\n");
    let logview = TextView::new_with_content(logbuf.clone());
    std::env::set_var("RUST_LOG", "ncspot=trace");
    std::env::set_var("RUST_BACKTRACE", "full");

    init_logger(logbuf);

    let mut cursive = Cursive::default();
    let event_manager = EventManager::new(cursive.cb_sink().clone());

    cursive.add_global_callback('q', |s| s.quit());
    cursive.set_theme(theme::default());

    let path = match env::var_os("HOME") {
        None => {
            println!("$HOME not set.");
            process::exit(1)
        }
        Some(path) => PathBuf::from(format!("{0}/.config/ncspot", path.into_string().unwrap())),
    };

    let cfg = config::load(path.to_str().unwrap()).expect("could not load configuration file");
    let queue = Arc::new(Mutex::new(queue::Queue::new(event_manager.clone())));

    let spotify = Arc::new(spotify::Spotify::new(
        event_manager.clone(),
        cfg.username,
        cfg.password,
        config::CLIENT_ID.to_string(),
        queue.clone(),
    ));

    // global player keybindings (play, pause, stop)
    {
        let spotify = spotify.clone();
        cursive.add_global_callback('P', move |_s| {
            spotify.toggleplayback();
        });
    }

    {
        let spotify = spotify.clone();
        cursive.add_global_callback('S', move |_s| {
            spotify.stop();
        });
    }

    {
        let spotify = spotify.clone();
        let queue = queue.clone();
        let event_manager = event_manager.clone();
        cursive.add_global_callback('>', move |_s| {
            let mut queue = queue.lock().unwrap();
            if let Some(track) = queue.dequeue() {
                let trackid = SpotifyId::from_base62(&track.id).expect("could not load track");
                spotify.load(trackid);
                spotify.play();
                event_manager.send(Event::QueueUpdate);
                event_manager.send(Event::SongChange(track));
            } else {
                spotify.stop();
            }
        });
    }

    let mut track_info = TextContent::new("");
    let mut playback_info = TextContent::new("");
    let counter = cursive::utils::Counter::new(0);

    let make_statusbar = || {
        LinearLayout::horizontal()
            .child(TextView::new_with_content(track_info.clone()).h_align(align::HAlign::Left))
            // .child(DummyView.full_width())
            .child(DummyView)
            .child(ProgressBar::new().with_value(counter.clone()).full_width())
            .child(DummyView)
            .child(TextView::new_with_content(playback_info.clone()).h_align(align::HAlign::Right))
    };

    let searchscreen = cursive.active_screen();
    let search = ui::search::SearchView::new(spotify.clone(), queue.clone());
    cursive.add_fullscreen_layer(
        LinearLayout::new(cursive::direction::Orientation::Vertical)
            .child(search.view)
            .child(make_statusbar()),
    );

    let queuescreen = cursive.add_active_screen();
    let mut queue_view = ui::queue::QueueView::new(queue.clone(), event_manager.clone());
    cursive.add_fullscreen_layer(
        LinearLayout::new(cursive::direction::Orientation::Vertical)
            .child(queue_view.view.take().unwrap())
            .child(make_statusbar()),
    );

    let logscreen = cursive.add_active_screen();
    let logview_scroller = ScrollView::new(logview).scroll_strategy(ScrollStrategy::StickToBottom);
    let logpanel = Panel::new(logview_scroller).title("Log");
    cursive.add_fullscreen_layer(
        LinearLayout::new(cursive::direction::Orientation::Vertical)
            .child(logpanel)
            .child(make_statusbar()),
    );

    let playlist_screen = cursive.add_active_screen();
    let playlist_view =
        ui::playlist::PlaylistView::new(spotify.clone(), queue.clone(), event_manager.clone());
    cursive.add_fullscreen_layer(
        LinearLayout::new(cursive::direction::Orientation::Vertical)
            .child(playlist_view.view)
            .child(make_statusbar()),
    );

    let screen_idx = Arc::new(AtomicUsize::new(0));

    {
        let screen_idx = screen_idx.clone();
        cursive.add_global_callback(Key::F4, move |s| {
            s.set_screen(logscreen);
            screen_idx.store(3, Ordering::Relaxed);
        });
    }

    {
        let event_manager = event_manager.clone();
        let screen_idx = screen_idx.clone();
        cursive.add_global_callback(Key::F2, move |s| {
            s.set_screen(queuescreen);
            screen_idx.store(1, Ordering::Relaxed);
            event_manager.clone().send(Event::QueueUpdate);
        });
    }

    {
        let screen_idx = screen_idx.clone();
        cursive.add_global_callback(Key::F3, move |s| {
            screen_idx.store(2, Ordering::Relaxed);
            s.set_screen(searchscreen);
        });
    }

    {
        let screen_idx = screen_idx.clone();
        cursive.add_global_callback(Key::F1, move |s| {
            screen_idx.store(0, Ordering::Relaxed);
            s.set_screen(playlist_screen);
        });
    }

    {
        let screen_idx = screen_idx.clone();
        let screen_order = vec![playlist_screen, queuescreen, searchscreen, logscreen];
        let event_manager = event_manager.clone();
        cursive.add_global_callback(Key::Tab, move |s| {
            let idx = screen_idx.fetch_add(1, Ordering::Relaxed);
            s.set_screen(screen_order[(idx + 1) % screen_order.len()]);
            event_manager.clone().send(Event::QueueUpdate);
        });
    }

    let fps = 60;

    cursive.set_fps(fps);

    let mut current_track: Option<FullTrack> = None;
    let mut ticks = 0;
    let mut increment_ticks = false;

    {
        let event_manager = event_manager.clone();
        cursive.add_global_callback('<', move |_s| {
            event_manager.send(Event::SeekTo(0));
        });
    }

    {
        let event_manager = event_manager.clone();
        cursive.add_global_callback(Key::Right, move |_s| {
            event_manager.send(Event::SeekForward(10_000));
        });
    }

    {
        let event_manager = event_manager.clone();
        cursive.add_global_callback(Key::Left, move |_s| {
            event_manager.send(Event::SeekBackward(10_000));
        });
    }

    // cursive event loop
    while cursive.is_running() {
        cursive.step();
        if ticks % fps == 0 {
            if let Some(ref current_track) = current_track {
                counter.set((ticks * 1000 / fps * 1_00 / current_track.duration_ms) as usize);
                playback_info.set_content(format!(
                    "{}:{:02} / {}:{:02}",
                    (ticks / fps) / 60,
                    (ticks / fps) % 60,
                    (current_track.duration_ms / 1000) / 60,
                    (current_track.duration_ms / 1000) % 60,
                ));
            }
        }
        if increment_ticks {
            ticks += 1;
        }

        for event in event_manager.msg_iter() {
            trace!("event received {}", event);
            match event {
                Event::QueueUpdate => {
                    queue_view.redraw(&mut cursive);
                    if spotify.is_stopped() && !queue.lock().unwrap().is_empty() {
                        event_manager.send(Event::CheckQueue);
                    }
                }
                Event::QueueAdd(track) => {
                    let mut queue = queue.lock().unwrap();
                    if queue.is_empty() && current_track.is_none() {
                        event_manager.send(Event::Play(track));
                    } else {
                        queue.enqueue(track);
                    }
                }
                Event::QueueRemove(i) => {
                    queue.lock().unwrap().remove(i);
                }
                Event::SongChange(track) => {
                    trace!("New track: {}", track.name);
                    // statusbar.set_content(format!("{}", track.name));
                    track_info.set_content(format!(
                        "{} - {}",
                        track
                            .artists
                            .iter()
                            .map(|a| a.name.clone())
                            .collect::<Vec<String>>()
                            .join(", "),
                        track.name,
                    ));
                    current_track = Some(track);
                    ticks = 0;
                }
                Event::PlayState(state) => {
                    match &state {
                        spotify::PlayerState::Playing => {
                            increment_ticks = true;
                        }
                        spotify::PlayerState::Paused => {
                            increment_ticks = false;
                        }
                        spotify::PlayerState::Stopped => {
                            current_track = None;
                            ticks = 0;
                            increment_ticks = false;
                        }
                    }
                    spotify.updatestate(state);
                }
                Event::Play(track) => {
                    use librespot::core::spotify_id::SpotifyId;

                    spotify.updatestate(spotify::PlayerState::Playing);
                    let trackid = SpotifyId::from_base62(&track.id).expect("could not load track");
                    spotify.load(trackid);
                    spotify.play();
                    event_manager.send(Event::SongChange(track));
                }
                Event::CheckQueue => {
                    spotify.check_queue();
                }
                Event::SeekTo(ms) => {
                    ticks = ms * fps / 1000;
                    spotify.seek_ms(ms);
                }
                Event::SeekForward(ms) => {
                    if let Some(ref current_track) = current_track {
                        let ms = std::cmp::min(ticks * 1000 / fps + ms, current_track.duration_ms);
                        ticks = ms * fps / 1000;
                        spotify.seek_ms(ms);
                    }
                }
                Event::SeekBackward(ms) => {
                    if let Some(_) = current_track {
                        let ms = std::cmp::max(ticks * 1000 / fps, ms) - ms;
                        ticks = ms * fps / 1000;
                        spotify.seek_ms(ms);
                    }
                }
            }
        }
    }
}
