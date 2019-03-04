use std::collections::vec_deque::Iter;
use std::collections::VecDeque;

use log::debug;

use rspotify::spotify::model::track::FullTrack;

use crate::events::{Event, EventManager};

pub struct Queue {
    queue: VecDeque<FullTrack>,
    ev: EventManager,
}

impl Queue {
    pub fn new(ev: EventManager) -> Queue {
        Queue {
            queue: VecDeque::new(),
            ev: ev,
        }
    }
    fn send_event(&self) {
        self.ev.send(Event::QueueUpdate);
    }

    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    pub fn clear(&mut self) {
        self.queue.clear();
        self.send_event();
    }

    pub fn remove(&mut self, index: usize) -> Option<FullTrack> {
        match self.queue.remove(index) {
            Some(track) => {
                debug!("Removed from queue: {}", &track.name);
                self.send_event();
                Some(track)
            }
            None => None,
        }
    }

    pub fn enqueue(&mut self, track: FullTrack) {
        debug!("Queued: {}", &track.name);
        self.queue.push_back(track);
        self.send_event();
    }

    pub fn dequeue(&mut self) -> Option<FullTrack> {
        match self.queue.pop_front() {
            Some(track) => {
                debug!("Dequeued : {}", track.name);
                self.send_event();
                Some(track)
            }
            None => None,
        }
    }

    pub fn iter(&self) -> Iter<FullTrack> {
        self.queue.iter()
    }
}
