use cursive::direction::Orientation;
use cursive::traits::Boxable;
use cursive::traits::Identifiable;
use cursive::views::*;
use cursive::Cursive;
use cursive_tree_view::{Placement, TreeView};
use derive_more::Display;
use std::sync::Arc;
use std::sync::Mutex;

use rspotify::spotify::model::playlist::{PlaylistTrack, SimplifiedPlaylist};

use crate::events::{Event, EventManager};
use crate::queue::Queue;
use crate::spotify::Spotify;

pub struct PlaylistView {
    pub view: Panel<LinearLayout>,
}

#[derive(Debug, Display, Clone)]
pub enum TreeEntry {
    #[display(fmt = "{}", "_0.name")]
    Playlist(SimplifiedPlaylist),
    #[display(
        fmt = "{} - {}",
        "_0.track.name",
        "_0.track.artists.iter().map(|a| a.name.clone()).collect::<Vec<String>>().join(\", \")"
    )]
    Track(PlaylistTrack),
}

const TREE_ID: &str = "playlist_tree";

impl PlaylistView {
    pub fn new(
        spotify: Arc<Spotify>,
        queue: Arc<Mutex<Queue>>,
        event_manager: EventManager,
    ) -> PlaylistView {
        let spotify = spotify.clone();

        let mut tree_view = TreeView::new();

        if let Ok(playlist_page) = spotify.api.current_user_playlists(None, None) {
            for (i, playlist) in playlist_page.items.into_iter().enumerate() {
                tree_view.insert_container_item(TreeEntry::Playlist(playlist), Placement::After, i);
            }
        }

        {
            let _queue = queue.clone();
            let _spotify = spotify.clone();
            tree_view.set_on_submit(move |siv: &mut Cursive, row: usize| {
                siv.call_on_id(TREE_ID, |tree_view: &mut TreeView<TreeEntry>| {
                    if let Some(&TreeEntry::Track(ref playlist_track)) = tree_view.borrow_item(row)
                    {
                        event_manager.send(Event::Play(playlist_track.track.clone()));

                        // // TODO send event to play instead.
                        // let trackid = SpotifyId::from_base62(&playlist_track.track.id)
                        //     .expect("could not load track");
                        // spotify.load(trackid);
                        // spotify.play();

                        // let mut queue = queue.lock().unwrap();
                        // queue.enqueue(playlist_track.track.clone());
                    }
                });
            });
        }

        tree_view.set_on_collapse(move |siv: &mut Cursive, row, is_collapsed, children| {
            if !is_collapsed && children == 0 {
                siv.call_on_id(TREE_ID, |tree_view: &mut TreeView<TreeEntry>| {
                    let track_page = if let Some(&TreeEntry::Playlist(ref playlist)) =
                        tree_view.borrow_item(row)
                    {
                        spotify
                            .api
                            .user_playlist_tracks(
                                &playlist.owner.id,
                                &playlist.id,
                                None,
                                100,
                                0,
                                None,
                            )
                            .ok()
                    } else {
                        None
                    };
                    for playlist_track in track_page.into_iter().flat_map(|t| t.items.into_iter()) {
                        tree_view.insert_item(
                            TreeEntry::Track(playlist_track),
                            Placement::LastChild,
                            row,
                        );
                    }
                });
            }
        });

        let view = {
            let queue = queue.clone();
            OnEventView::new(tree_view.with_id(TREE_ID)).on_event('a', move |siv: &mut Cursive| {
                siv.call_on_id(TREE_ID, |tree_view: &mut TreeView<TreeEntry>| {
                    if let Some(focused_row) = tree_view.row() {
                        match tree_view.borrow_item(focused_row) {
                            Some(&TreeEntry::Playlist(_)) => {
                                let mut idx = focused_row + 1;
                                let mut queue = queue.lock().unwrap();
                                while let Some(&TreeEntry::Track(ref playlist_track)) =
                                    tree_view.borrow_item(idx)
                                {
                                    queue.enqueue(playlist_track.track.clone());
                                    idx += 1;
                                }
                            }
                            Some(&TreeEntry::Track(ref playlist_track)) => {
                                let mut queue = queue.lock().unwrap();
                                queue.enqueue(playlist_track.track.clone());
                            }
                            None => (),
                        }
                    }
                });
            })
        };

        let layout = LinearLayout::new(Orientation::Vertical)
            // .child(searchfield)
            .child(view.full_width());

        //         let searchfield = EditView::new()
        //             .on_submit(move |s, input| {
        //                 PlaylistView::search_handler(s, input, spotify_ref.clone(), queue_ref.clone());
        //             })
        //             .with_id("search_edit")
        //             .full_width()
        //             .fixed_height(1);
        //         let results = ListView::new().with_id("search_results").full_width();
        //         let scrollable = ScrollView::new(results).full_width().full_height();

        let rootpanel = Panel::new(layout).title("Playlists");
        PlaylistView { view: rootpanel }
    }
}
