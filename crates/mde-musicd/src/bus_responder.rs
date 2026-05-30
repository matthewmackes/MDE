//! AIR-2 (v6.1) — Bus-native control surface for the music daemon.
//!
//! Per the Q96 Bus-canonical lock (EPIC-RETIRE-DBUS), the daemon's
//! MDE-internal control is **Bus**, not a new `dev.mackes.MDE.Music`
//! D-Bus interface. The GUI (and `mde-bus publish`) send requests on
//! `action/music/<verb>`; the responder applies them to the shared
//! [`Queue`] and writes the result to `reply/<request-ulid>`. (MPRIS
//! `org.mpris.MediaPlayer2` — FDO-standard — stays D-Bus for media-key /
//! lock-screen interop; that + the play flow are AIR-2.c, gated on the
//! AIR-5 audio engine.)
//!
//! The verb dispatch ([`dispatch_queue_action`]) is a pure function over
//! the [`Queue`], fully unit-testable; [`serve`] is the thin poll loop
//! (modeled on `mackesd::workers::marks_state`) that drives it off the
//! Bus persistence store.

use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;

use mde_bus::hooks::config::Priority;
use mde_bus::persist::Persist;
use mde_bus::rpc::reply_topic;
use serde_json::json;

use crate::airsonic::Client;
use crate::creds;
use crate::queue::{self, Queue};

/// Poll cadence for the action topics.
pub const POLL_INTERVAL: Duration = Duration::from_millis(500);

/// The queue-control verbs served on `action/music/<verb>` (synchronous
/// — they only touch the local queue file).
pub const ACTION_VERBS: [&str; 6] =
    ["enqueue", "enqueue-after", "clear", "next", "prev", "get-queue"];

/// The library-browse verbs served on `action/music/<verb>`
/// (asynchronous — each proxies an Airsonic REST call).
pub const BROWSE_VERBS: [&str; 3] = ["list-albums", "list-artists", "search"];

/// Result of dispatching one action: the JSON reply + whether the queue
/// changed (and so must be persisted).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dispatch {
    /// JSON written to `reply/<request-ulid>`.
    pub reply_json: String,
    /// Whether the queue changed and must be persisted.
    pub mutated: bool,
}

/// Extract a song-id from a request body: either a bare string or
/// `{"song_id": "..."}`.
#[must_use]
fn song_id_from(body: &str) -> Option<String> {
    let trimmed = body.trim();
    if trimmed.is_empty() {
        return None;
    }
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(trimmed) {
        if let Some(s) = v.get("song_id").and_then(serde_json::Value::as_str) {
            return Some(s.to_string());
        }
        if let Some(s) = v.as_str() {
            return Some(s.to_string());
        }
    }
    // Fall back to the raw body as the id.
    Some(trimmed.trim_matches('"').to_string())
}

fn queue_reply(q: &Queue, mutated: bool) -> Dispatch {
    Dispatch {
        reply_json: json!({
            "ok": true,
            "len": q.len(),
            "current": q.current(),
            "songs": q.songs,
        })
        .to_string(),
        mutated,
    }
}

fn error_reply(message: &str) -> Dispatch {
    Dispatch {
        reply_json: json!({ "ok": false, "error": message }).to_string(),
        mutated: false,
    }
}

/// Apply one `action/music/<verb>` request to `q`, returning the reply.
#[must_use]
pub fn dispatch_queue_action(verb: &str, body: &str, q: &mut Queue) -> Dispatch {
    match verb {
        "enqueue" => match song_id_from(body) {
            Some(id) => {
                q.enqueue(id);
                queue_reply(q, true)
            }
            None => error_reply("enqueue: missing song_id"),
        },
        "enqueue-after" => match song_id_from(body) {
            Some(id) => {
                q.enqueue_after_current(id);
                queue_reply(q, true)
            }
            None => error_reply("enqueue-after: missing song_id"),
        },
        "clear" => {
            q.clear();
            queue_reply(q, true)
        }
        "next" => {
            q.next();
            queue_reply(q, true)
        }
        "prev" => {
            q.prev();
            queue_reply(q, true)
        }
        "get-queue" => queue_reply(q, false),
        other => error_reply(&format!("unknown verb: {other}")),
    }
}

/// Reply JSON for a library-browse verb. Proxies the Airsonic REST call
/// via the shared [`Client`]; missing creds / server errors become an
/// `{ok:false,error}` reply rather than a panic. I/O, so not pure — the
/// URL-building + parse logic it leans on is unit-tested in [`crate::airsonic`].
fn dispatch_browse(verb: &str, body: &str, client: &Client, rt: &tokio::runtime::Runtime) -> String {
    let result: Result<serde_json::Value, String> = rt.block_on(async {
        match verb {
            "list-albums" => client
                .get_album_list2("newest", 100)
                .await
                .map(|a| json!({ "albums": a }))
                .map_err(|e| e.to_string()),
            "list-artists" => client
                .get_artists()
                .await
                .map(|a| json!({ "artists": a }))
                .map_err(|e| e.to_string()),
            "search" => {
                let query = song_id_from(body).unwrap_or_default();
                client
                    .search3(&query)
                    .await
                    .map(|r| json!({ "artists": r.artists, "albums": r.albums, "songs": r.songs }))
                    .map_err(|e| e.to_string())
            }
            other => Err(format!("unknown browse verb: {other}")),
        }
    });
    match result {
        Ok(v) => json!({ "ok": true, "result": v }).to_string(),
        Err(e) => json!({ "ok": false, "error": e }).to_string(),
    }
}

/// One browse-poll sweep: for each browse verb, dispatch new requests
/// against a freshly-built Airsonic client. A missing-creds state replies
/// with an error (the GUI prompts the operator to connect).
pub fn poll_browse(
    persist: &Persist,
    rt: &tokio::runtime::Runtime,
    cursors: &mut HashMap<String, String>,
) {
    let client = creds::load().ok().map(|c| Client::new(&c.server_url, &c.username, &c.password));
    for verb in BROWSE_VERBS {
        let topic = format!("action/music/{verb}");
        let since = cursors.get(&topic).map(String::as_str);
        let msgs = match persist.list_since(&topic, since) {
            Ok(m) => m,
            Err(_) => continue,
        };
        for msg in msgs {
            cursors.insert(topic.clone(), msg.ulid.clone());
            let reply = match &client {
                Some(c) => dispatch_browse(verb, msg.body.as_deref().unwrap_or(""), c, rt),
                None => json!({ "ok": false, "error": "no Airsonic server configured" }).to_string(),
            };
            let _ = persist.write(&reply_topic(&msg.ulid), Priority::Default, None, Some(&reply));
        }
    }
}

/// Run the Bus responder loop: poll the queue-control + library-browse
/// `action/music/<verb>` topics for new requests, dispatch them, and
/// reply on `reply/<ulid>`. Loops until `should_stop()` returns true.
pub fn serve<F: Fn() -> bool>(persist: &Persist, queue_path: &Path, should_stop: F) {
    let mut cursors: HashMap<String, String> = HashMap::new();
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("tokio runtime for browse proxy");
    while !should_stop() {
        poll_once(persist, queue_path, &mut cursors);
        poll_browse(persist, &rt, &mut cursors);
        std::thread::sleep(POLL_INTERVAL);
    }
}

/// One poll sweep across the action verbs (extracted so a test can drive
/// it deterministically without the sleep loop).
pub fn poll_once(persist: &Persist, queue_path: &Path, cursors: &mut HashMap<String, String>) {
    let mut q = queue::read_from(queue_path);
    for verb in ACTION_VERBS {
        let topic = format!("action/music/{verb}");
        let since = cursors.get(&topic).map(String::as_str);
        let msgs = match persist.list_since(&topic, since) {
            Ok(m) => m,
            Err(_) => continue,
        };
        for msg in msgs {
            cursors.insert(topic.clone(), msg.ulid.clone());
            let d = dispatch_queue_action(verb, msg.body.as_deref().unwrap_or(""), &mut q);
            let _ = persist.write(
                &reply_topic(&msg.ulid),
                Priority::Default,
                None,
                Some(&d.reply_json),
            );
            if d.mutated {
                let _ = queue::write_to(queue_path, &q);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn song_id_parsing_forms() {
        assert_eq!(song_id_from(r#"{"song_id":"s1"}"#).as_deref(), Some("s1"));
        assert_eq!(song_id_from(r#""s2""#).as_deref(), Some("s2"));
        assert_eq!(song_id_from("s3").as_deref(), Some("s3"));
        assert_eq!(song_id_from("  "), None);
    }

    #[test]
    fn dispatch_enqueue_and_get() {
        let mut q = Queue::default();
        let d = dispatch_queue_action("enqueue", r#"{"song_id":"a"}"#, &mut q);
        assert!(d.mutated);
        assert!(d.reply_json.contains("\"ok\":true"));
        assert!(d.reply_json.contains("\"len\":1"));
        // get-queue doesn't mutate.
        let g = dispatch_queue_action("get-queue", "", &mut q);
        assert!(!g.mutated);
        assert!(g.reply_json.contains("\"current\":\"a\""));
    }

    #[test]
    fn dispatch_enqueue_after_and_walk() {
        let mut q = Queue::default();
        let _ = dispatch_queue_action("enqueue", "a", &mut q);
        let _ = dispatch_queue_action("enqueue", "b", &mut q);
        let _ = dispatch_queue_action("enqueue-after", "x", &mut q);
        assert_eq!(q.songs, vec!["a", "x", "b"]);
        let d = dispatch_queue_action("next", "", &mut q);
        assert!(d.mutated);
        assert_eq!(q.current(), Some("x"));
    }

    #[test]
    fn poll_once_round_trips_a_request() {
        let dir = tempfile::tempdir().unwrap();
        let persist = Persist::open(dir.path().join("bus")).unwrap();
        let queue_path = dir.path().join("queue.json");
        // A GUI publishes an enqueue request on the action topic.
        let req = persist
            .write(
                "action/music/enqueue",
                Priority::Default,
                None,
                Some(r#"{"song_id":"t1"}"#),
            )
            .unwrap();
        let mut cursors = HashMap::new();
        poll_once(&persist, &queue_path, &mut cursors);
        // A reply landed on reply/<ulid> with ok:true.
        let replies = persist.list_since(&reply_topic(&req.ulid), None).unwrap();
        assert_eq!(replies.len(), 1);
        assert!(replies[0].body.as_deref().unwrap().contains("\"ok\":true"));
        // The queue was persisted with the enqueued track.
        assert_eq!(queue::read_from(&queue_path).songs, vec!["t1"]);
        // A second poll with the advanced cursor does nothing new.
        poll_once(&persist, &queue_path, &mut cursors);
        assert_eq!(
            persist.list_since(&reply_topic(&req.ulid), None).unwrap().len(),
            1
        );
    }

    #[test]
    fn dispatch_clear_and_errors() {
        let mut q = Queue::default();
        let _ = dispatch_queue_action("enqueue", "a", &mut q);
        let c = dispatch_queue_action("clear", "", &mut q);
        assert!(c.mutated);
        assert!(q.is_empty());
        // Missing id.
        let e = dispatch_queue_action("enqueue", "", &mut q);
        assert!(e.reply_json.contains("\"ok\":false"));
        // Unknown verb.
        let u = dispatch_queue_action("frobnicate", "", &mut q);
        assert!(u.reply_json.contains("unknown verb"));
    }
}
