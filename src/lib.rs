// Copyright (c) 2019 libechoexec developers
//
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or http://www.apache.org/licenses/LICENSE-2.0> or the MIT
// license <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. All files in the project carrying such notice may not be copied,
// modified, or distributed except according to those terms.

//! An Echo client over a `tokio` runtime to ease asynchronous processing
//!
//! # Examples
//!
//! ## Single Thread
//! ```
//! # use {
//! #     chrono::Utc,
//! #     libechoexec::{Event, EventType, Payload, Result, Spawner},
//! #     std::{time::Duration, thread},
//! #     uuid::Uuid,
//! # };
//! # fn main() -> Result<()> {
//!       // Setup the event spawner
//!       let echo_spawner = Spawner::new()?;
//!
//!       // Create an 'Echo Event'
//!       let mut echo_event = Event::default();
//!       let _ = echo_event.set_routing_key("atlas-dev-promises");
//!       let _ = echo_event.set_event_type(EventType::System);
//!       let _ = echo_event.set_message("testing");
//!       let _ = echo_event.set_correlation_id(Some(Uuid::parse_str(
//!           "35F3E1D6-D859-4AA0-8C58-2CDFE97A4710",
//!       )?));
//!       let _ = echo_event.set_timestamp(Some(Utc::now().timestamp_millis()));
//!
//!       // Setup the payload
//!       let mut payload = Payload::default();
//!       let _ = payload.set_events(vec![echo_event]);
//!
//!       // Spawn the payload onto the runtime to be handled asynchronously
//!       assert!(echo_spawner.spawn(&payload).is_ok());
//!
//!       // Sleep so the spawned future can complete in this short example
//!       thread::sleep(Duration::from_millis(500));
//! #    Ok(())
//! # }
//! ```
//!
//! ## Multi Threaded
//! ```
//! # use {
//! #     chrono::Utc,
//! #     libechoexec::{Event, EventType, Payload, Result, Spawner},
//! #     std::{sync::mpsc::channel, time::Duration, thread},
//! #     uuid::Uuid,
//! # };
//! #
//! # fn multi_thread_spawn() -> Result<()> {
//!       // Setup the event spawner
//!       let echo_spawner = Spawner::new()?;
//!       let (tx, rx) = channel();
//!       let mut handles = vec![];
//!
//!       // Create an 'Echo Event'
//!       let mut echo_event = Event::default();
//!       let _ = echo_event.set_routing_key("atlas-local-promises");
//!       let _ = echo_event.set_event_type(EventType::System);
//!       let _ = echo_event.set_message("testing");
//!       let _ = echo_event.set_correlation_id(Some(Uuid::parse_str(
//!           "35F3E1D6-D859-4AA0-8C58-2CDFE97A4710",
//!       )?));
//!       let _ = echo_event.set_timestamp(Some(Utc::now().timestamp_millis()));
//!
//!       // Spin up 10 threads, and give each a transmitter
//!       for _ in 0..10 {
//!           let tx = tx.clone();
//!           handles.push(thread::spawn(move || {
//!               tx.send("message").expect("unable to send message");
//!           }));
//!       }
//!
//!       // Listen for 10 messages on the receiver end
//!       // Spawn a payload onto the runtime each time a message is received
//!       for _ in 0..10 {
//!           let j = rx.recv().map_err(|e| format!("{}", e))?;
//!           assert_eq!(j, "message");
//!           let mut payload = Payload::default();
//!           let _ = payload.set_events(vec![echo_event.clone()]);
//!           let _ = echo_spawner.spawn(&payload);
//!       }
//!
//!       // Sleep so the spawned futures can complete
//!       thread::sleep(Duration::from_millis(500));
//!
//!       for handle in handles {
//!           let _ = handle.join();
//!       }
//! #     Ok(())
//! # }
//! ```
#![feature(async_await, crate_visibility_modifier)]
#![deny(
    absolute_paths_not_starting_with_crate,
    anonymous_parameters,
    bare_trait_objects,
    clippy::all,
    clippy::pedantic,
    dead_code,
    elided_lifetimes_in_paths,
    ellipsis_inclusive_range_patterns,
    keyword_idents,
    macro_use_extern_crate,
    missing_copy_implementations,
    missing_debug_implementations,
    missing_docs,
    single_use_lifetimes,
    trivial_casts,
    trivial_numeric_casts,
    unreachable_pub,
    unsafe_code,
    unused,
    unused_import_braces,
    unused_labels,
    unused_lifetimes,
    unused_qualifications,
    unused_results,
    variant_size_differences
)]
#![allow(box_pointers)]
#![doc(html_root_url = "https://docs.rs/echoloc/0.1.0")]

mod echo;
mod error;

pub use {
    echo::{CollectorUrl, Event, EventType, Payload, Response, Spawner},
    error::{Err, ErrKind, Result},
};
