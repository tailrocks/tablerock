use tablerock_cli::{Delivery, SendOutcome, TryReceiveError, bounded_ingress};

#[tokio::test]
async fn overflow_is_reported_before_surviving_state_transitions() {
    let (sender, mut receiver) = bounded_ingress::<i32, ()>(2);
    assert_eq!(sender.try_send_event(10), Ok(SendOutcome::Accepted));
    assert_eq!(sender.try_send_event(20), Ok(SendOutcome::Accepted));
    assert_eq!(sender.try_send_event(30), Ok(SendOutcome::ResyncRequired));

    assert_eq!(receiver.recv().await, Some(Delivery::ResyncRequired));
    assert_eq!(receiver.recv().await, Some(Delivery::Event(10)));
    assert_eq!(receiver.recv().await, Some(Delivery::Event(20)));
}

#[test]
fn progress_is_latest_only_and_never_precedes_state() {
    let (sender, mut receiver) = bounded_ingress::<&str, i32>(1);
    sender.publish_progress(1).expect("open ingress");
    sender.publish_progress(2).expect("coalesce progress");
    sender.publish_progress(3).expect("coalesce progress");
    assert_eq!(
        sender.try_send_event("terminal outcome"),
        Ok(SendOutcome::Accepted)
    );

    assert_eq!(receiver.try_recv(), Ok(Delivery::Event("terminal outcome")));
    assert_eq!(receiver.try_recv(), Ok(Delivery::Progress(3)));
    assert_eq!(receiver.try_recv(), Err(TryReceiveError::Empty));
}

#[tokio::test]
async fn close_drains_accepted_work_then_ends_stream() {
    let (sender, mut receiver) = bounded_ingress(1);
    sender.publish_progress(7).expect("open ingress");
    sender.try_send_event(8).expect("accepted event");
    drop(sender);

    assert_eq!(receiver.recv().await, Some(Delivery::Event(8)));
    assert_eq!(receiver.recv().await, Some(Delivery::Progress(7)));
    assert_eq!(receiver.recv().await, None);
    assert_eq!(receiver.try_recv(), Err(TryReceiveError::Closed));
}

#[test]
fn closed_receiver_returns_the_undelivered_value() {
    let (sender, receiver) = bounded_ingress::<i32, i32>(1);
    drop(receiver);

    assert_eq!(sender.try_send_event(42), Err(42));
    assert_eq!(sender.publish_progress(43), Err(43));
}

#[test]
fn high_rate_progress_and_repeated_overflow_collapse_without_starving_state() {
    let (sender, mut receiver) = bounded_ingress(1);
    for progress in 0..10_000 {
        sender
            .publish_progress(progress)
            .expect("coalesced progress remains bounded");
    }
    assert_eq!(sender.try_send_event(10_001), Ok(SendOutcome::Accepted));
    for event in 10_002..20_000 {
        assert_eq!(
            sender.try_send_event(event),
            Ok(SendOutcome::ResyncRequired)
        );
    }

    assert_eq!(receiver.try_recv(), Ok(Delivery::ResyncRequired));
    assert_eq!(receiver.try_recv(), Ok(Delivery::Event(10_001)));
    assert_eq!(receiver.try_recv(), Ok(Delivery::Progress(9_999)));
    assert_eq!(receiver.try_recv(), Err(TryReceiveError::Empty));
}

#[test]
fn concurrent_receiver_close_is_linearizable_with_progress_publication() {
    use std::{sync::Barrier, thread};

    for value in 0..1_000 {
        let (sender, receiver) = bounded_ingress::<(), i32>(1);
        let barrier = std::sync::Arc::new(Barrier::new(2));
        let publisher_barrier = std::sync::Arc::clone(&barrier);
        let publisher = thread::spawn(move || {
            publisher_barrier.wait();
            sender.publish_progress(value)
        });
        barrier.wait();
        drop(receiver);
        if let Err(returned) = publisher.join().expect("join publisher") {
            assert_eq!(returned, value);
        }
    }
}

#[test]
fn concurrent_event_and_progress_publication_preserve_class_priority() {
    use std::{sync::Barrier, thread};

    for value in 0..1_000 {
        let (sender, mut receiver) = bounded_ingress::<i32, i32>(1);
        let progress_sender = sender.clone();
        let barrier = std::sync::Arc::new(Barrier::new(3));
        let event_barrier = std::sync::Arc::clone(&barrier);
        let event = thread::spawn(move || {
            event_barrier.wait();
            sender.try_send_event(value)
        });
        let progress_barrier = std::sync::Arc::clone(&barrier);
        let progress = thread::spawn(move || {
            progress_barrier.wait();
            progress_sender.publish_progress(value)
        });
        barrier.wait();
        assert_eq!(event.join().expect("join event"), Ok(SendOutcome::Accepted));
        assert_eq!(progress.join().expect("join progress"), Ok(()));
        assert_eq!(receiver.try_recv(), Ok(Delivery::Event(value)));
        assert_eq!(receiver.try_recv(), Ok(Delivery::Progress(value)));
    }
}
