use cursive::event::Key;
use cursive::traits::Boxable;
use cursive::traits::Identifiable;
use cursive::traits::Scrollable;
use cursive::views::*;
use cursive::Cursive;

use std::sync::Arc;
use std::sync::Mutex;

use crate::events::{Event, EventManager};
use crate::queue::Queue;

pub struct QueueView {
    pub view: OnEventView<Panel<LinearLayout>>,
}

const QUEUE_ID: &str = "queue_list";

impl QueueView {
    pub fn new(queue: Arc<Mutex<Queue>>, event_manager: EventManager) -> QueueView {
        // let queuelist = ListView::new().with_id(QUEUE_ID);
        let queuelist = SelectView::<String>::new().with_id(QUEUE_ID);

        let layout = LinearLayout::vertical().child(
            queuelist
                .full_width()
                .scrollable()
                .full_width()
                .full_height(),
        );
        let panel = Panel::new(layout).title("Queue");

        let mut queuelist = OnEventView::new(panel);

        {
            let queue = queue.clone();
            // <d> removes the selected track without playing it.
            queuelist.set_on_event('c', move |_cursive| {
                queue.lock().unwrap().clear();
            });
        }

        {
            let queue = queue.clone();
            let event_manager = event_manager.clone();
            // <enter> dequeues the selected track
            queuelist.set_on_pre_event(Key::Enter, move |siv| {
                siv.call_on_id(QUEUE_ID, |queuelist: &mut SelectView| {
                    let selected_id = queuelist.selected_id().unwrap();
                    let track = queue
                        .lock()
                        .unwrap()
                        .remove(selected_id)
                        .expect("could not dequeue track");
                    event_manager.send(Event::Play(track));
                    event_manager.send(Event::QueueUpdate);
                });
            });
        }

        {
            let queue = queue.clone();
            let event_manager = event_manager.clone();
            // <d> removes the selected track without playing it.
            queuelist.set_on_event('d', move |siv| {
                siv.call_on_id(QUEUE_ID, |queuelist: &mut SelectView| {
                    let selected_id = queuelist.selected_id().unwrap();
                    queue
                        .lock()
                        .unwrap()
                        .remove(selected_id)
                        .expect("could not dequeue track");
                    event_manager.send(Event::QueueUpdate);
                });
            });
        }

        QueueView { view: queuelist }
    }

    pub fn redraw(s: &mut Cursive, queue: Arc<Mutex<Queue>>) {
        s.call_on_id(QUEUE_ID, |queuelist: &mut SelectView| {
            let selected_id = queuelist.selected_id();
            queuelist.clear();

            let queue = queue.lock().unwrap();
            for track in queue.iter() {
                let label = format!(
                    "{} - {}",
                    track.name,
                    track
                        .artists
                        .iter()
                        .map(|a| a.name.clone())
                        .collect::<Vec<String>>()
                        .join(", ")
                );
                queuelist.add_item(label, String::new());
            }

            if let Some(selected_id) = selected_id {
                queuelist.set_selection(selected_id);
            }
        });
    }
}
