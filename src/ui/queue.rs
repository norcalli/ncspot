use cursive::direction::Orientation;
use cursive::event::Key;
use cursive::traits::Boxable;
use cursive::traits::Identifiable;
use cursive::views::*;
use cursive::Cursive;

use std::sync::Arc;
use std::sync::Mutex;

use librespot::core::spotify_id::SpotifyId;

use rspotify::spotify::model::track::FullTrack;

use crate::events::{Event, EventManager};
use crate::queue::Queue;
use crate::spotify::{PlayerState, Spotify};
use crate::ui::trackbutton::TrackButton;

pub struct QueueView {
    pub view: Option<Panel<LinearLayout>>,
    queue: Arc<Mutex<Queue>>,
}

const QUEUE_ID: &str = "queue_list";

impl QueueView {
    pub fn new(queue: Arc<Mutex<Queue>>, event_manager: EventManager) -> QueueView {
        let mut queuelist = OnEventView::new(ListView::new().with_id(QUEUE_ID));

        {
            let queue = queue.clone();
            // <d> removes the selected track without playing it.
            queuelist.set_on_pre_event('c', move |_cursive| {
                queue.lock().unwrap().clear();
            });
        }

        {
            let queue = queue.clone();
            let event_manager = event_manager.clone();
            // <enter> dequeues the selected track
            queuelist.set_on_pre_event(Key::Enter, move |siv| {
                siv.call_on_id(QUEUE_ID, |queuelist: &mut ListView| {
                    let track = queue
                        .lock()
                        .unwrap()
                        .remove(queuelist.focus())
                        .expect("could not dequeue track");
                    event_manager.send(Event::SongChange(track));
                    event_manager.send(Event::QueueUpdate);
                });
            });
        }

        {
            let queue = queue.clone();
            let event_manager = event_manager.clone();
            // <d> removes the selected track without playing it.
            queuelist.set_on_pre_event('d', move |siv| {
                siv.call_on_id(QUEUE_ID, |queuelist: &mut ListView| {
                    queue
                        .lock()
                        .unwrap()
                        .remove(queuelist.focus())
                        .expect("could not dequeue track");
                    event_manager.send(Event::QueueUpdate);
                });
            });
        }

        let scrollable = ScrollView::new(queuelist.full_width())
            .full_width()
            .full_height();
        let layout = LinearLayout::new(Orientation::Vertical).child(scrollable);
        let panel = Panel::new(layout).title("Queue");

        QueueView {
            view: Some(panel),
            queue,
        }
    }

    pub fn redraw(&self, s: &mut Cursive) {
        s.call_on_id(QUEUE_ID, |queuelist: &mut ListView| {
            queuelist.clear();

            let queue = self.queue.lock().unwrap();
            for track in queue.iter() {
                queuelist.add_child(
                    "",
                    TextView::new(format!(
                        "{} - {}",
                        track.name,
                        track
                            .artists
                            .iter()
                            .map(|a| a.name.clone())
                            .collect::<Vec<String>>()
                            .join(", ")
                    )),
                );
            }
        });
    }
}
