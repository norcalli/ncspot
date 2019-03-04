use crossbeam_channel::{unbounded, Receiver, Sender, TryIter};
use cursive::{CbFunc, Cursive};
use derive_more::Display;

use crate::spotify::PlayerState;

use rspotify::spotify::model::track::FullTrack;

#[derive(Display)]
pub enum Event {
    #[display(fmt = "Event::QueueUpdate")]
    QueueUpdate,
    #[display(fmt = "Event::PlayState")]
    PlayState(PlayerState),
    #[display(fmt = "Event::Play")]
    Play(FullTrack),
    #[display(fmt = "Event::CheckQueue")]
    CheckQueue,
    #[display(fmt = "Event::SeekTo")]
    SeekTo(u32),
    #[display(fmt = "Event::SeekForward")]
    SeekForward(u32),
    #[display(fmt = "Event::SeekBackward")]
    SeekBackward(u32),
    #[display(fmt = "Event::QueueAdd")]
    QueueAdd(FullTrack),
    #[display(fmt = "Event::QueueRemove")]
    QueueRemove(usize),
    #[display(fmt = "Event::SongChange")]
    SongChange(FullTrack),
    // #[display(fmt = "Event::SongFinish")]
    // SongFinish,
}

pub type EventSender = Sender<Event>;

#[derive(Clone)]
pub struct EventManager {
    tx: EventSender,
    rx: Receiver<Event>,
    cursive_sink: Sender<Box<dyn CbFunc>>,
}

impl EventManager {
    pub fn new(cursive_sink: Sender<Box<dyn CbFunc>>) -> EventManager {
        let (tx, rx) = unbounded();

        EventManager {
            tx: tx,
            rx: rx,
            cursive_sink: cursive_sink,
        }
    }

    pub fn msg_iter(&self) -> TryIter<Event> {
        self.rx.try_iter()
    }

    pub fn send(&self, event: Event) {
        self.tx.send(event).expect("could not send event");
        self.trigger();
    }

    pub fn trigger(&self) {
        // send a no-op to trigger event loop processing
        self.cursive_sink
            .send(Box::new(|_s: &mut Cursive| {}))
            .expect("could not send no-op event to cursive");
    }
}
