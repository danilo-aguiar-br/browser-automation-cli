// SPDX-License-Identifier: MIT OR Apache-2.0

use std::sync::Arc;

use futures::stream;
use serde::Serialize;
use tokio::sync::broadcast;

use super::forwarders::spawn_cdp_event_forwarder;

#[derive(Debug, Serialize)]
struct DummyEvent {
    n: u32,
}

#[tokio::test]
async fn cdp_event_forwarder_serializes_and_publishes() {
    let (tx, mut rx) = broadcast::channel(4);
    let stream = stream::iter(vec![Arc::new(DummyEvent { n: 7 })]);
    let handle = spawn_cdp_event_forwarder(stream, "Test.event", tx, None);
    let ev = rx.recv().await.expect("event delivered");
    assert_eq!(ev.method, "Test.event");
    assert_eq!(ev.params["n"], 7);
    assert!(ev.session_id.is_none());
    handle.await.expect("forwarder task");
}

#[tokio::test]
async fn cdp_event_forwarder_stamps_page_session_id() {
    let (tx, mut rx) = broadcast::channel(4);
    let stream = stream::iter(vec![Arc::new(DummyEvent { n: 1 })]);
    let handle = spawn_cdp_event_forwarder(
        stream,
        "Page.javascriptDialogClosed",
        tx,
        Some("sess-tab-b".into()),
    );
    let ev = rx.recv().await.expect("event delivered");
    assert_eq!(ev.session_id.as_deref(), Some("sess-tab-b"));
    handle.await.expect("forwarder task");
}
