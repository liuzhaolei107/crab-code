//! TUI event system — merges crossterm terminal events with agent domain events.

use std::sync::Arc;

use crab_core::event::Event as AgentEvent;
use crossterm::event::{Event as CtEvent, KeyEvent, KeyEventKind};
use tokio::sync::mpsc;

use crate::event_broker::EventBroker;

/// Sentinel epoch for events that should always be considered current
/// (i.e. never filtered out by the REPL's stale-event check).
///
/// Used by tests and by any sender that does not yet have a real query
/// epoch to attach.
pub const ALWAYS_CURRENT_EPOCH: u64 = u64::MAX;

/// Events consumed by the TUI main loop.
#[derive(Debug)]
pub enum TuiEvent {
    /// Keyboard event from the terminal.
    Key(KeyEvent),
    /// Terminal resize.
    Resize { width: u16, height: u16 },
    /// Agent domain event (tool use, content delta, etc.) tagged with the
    /// query epoch it originated from. Stale events from a cancelled query
    /// can be discarded by comparing `epoch` against the REPL's current
    /// query epoch. Permission and other channel events that are not
    /// tied to a single query use [`ALWAYS_CURRENT_EPOCH`].
    Agent { epoch: u64, event: AgentEvent },
    /// Bracketed paste from the terminal.
    Paste(String),
    /// Periodic tick for animations (spinner, etc.).
    Tick,
}

/// Spawn the event loop that merges crossterm and agent events into a single channel.
///
/// Returns the receiver. The caller should also hold the `agent_tx` sender and
/// forward `(epoch, AgentEvent)` pairs into it from the agent task.
///
/// `broker` controls whether crossterm key/resize events reach the receiver. When
/// `broker.is_paused()` is true (e.g. while an external editor owns the terminal),
/// terminal events are dropped instead of being forwarded. Tick events still fire
/// regardless so the app can keep rendering.
///
/// Buffering choice: dropped, not queued. Buffering keystrokes that the user typed
/// "blind" while the editor was attached would replay them into the TUI on resume,
/// which is surprising and almost never what the user wants.
///
/// The loop runs until the returned receiver is dropped.
pub fn spawn_event_loop(
    agent_rx: mpsc::UnboundedReceiver<(u64, AgentEvent)>,
    tick_rate: std::time::Duration,
    broker: Arc<EventBroker>,
) -> mpsc::UnboundedReceiver<TuiEvent> {
    let (tx, rx) = mpsc::unbounded_channel();

    // Crossterm reader task — owns the broker handle. Caller passes a freshly
    // cloned `Arc` and retains its own clone for pause/resume.
    let ct_tx = tx.clone();
    tokio::spawn(async move {
        use futures::StreamExt;
        let mut reader = crossterm::event::EventStream::new();
        while let Some(Ok(event)) = reader.next().await {
            // Drop terminal events while the broker is paused — the external
            // process owns the terminal and any input belongs to it.
            if broker.is_paused() {
                continue;
            }
            let tui_event = match event {
                // Only handle Press events — Windows reports both Press and
                // Release, which would double every keystroke / IME character.
                CtEvent::Key(key) if key.kind == KeyEventKind::Press => TuiEvent::Key(key),
                CtEvent::Resize(w, h) => TuiEvent::Resize {
                    width: w,
                    height: h,
                },
                CtEvent::Paste(text) => TuiEvent::Paste(text),
                // Ignore Key Release/Repeat, mouse, and focus events.
                _ => continue,
            };
            if ct_tx.send(tui_event).is_err() {
                break;
            }
        }
    });

    // Agent event forwarder task
    let agent_tx = tx.clone();
    tokio::spawn(async move {
        let mut agent_rx = agent_rx;
        while let Some((epoch, event)) = agent_rx.recv().await {
            if agent_tx.send(TuiEvent::Agent { epoch, event }).is_err() {
                break;
            }
        }
    });

    // Tick timer task
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tick_rate);
        loop {
            interval.tick().await;
            if tx.send(TuiEvent::Tick).is_err() {
                break;
            }
        }
    });

    rx
}

/// A sender that tags every outbound `AgentEvent` with a fixed epoch so the
/// REPL can later discard events from a cancelled query.
///
/// Construct via [`EpochTaggedSender::new`] and pass into the agent runtime
/// in place of a raw `mpsc::Sender<Event>`.
#[derive(Clone)]
pub struct EpochTaggedSender {
    inner: mpsc::Sender<(u64, AgentEvent)>,
    epoch: u64,
}

impl EpochTaggedSender {
    pub fn new(inner: mpsc::Sender<(u64, AgentEvent)>, epoch: u64) -> Self {
        Self { inner, epoch }
    }

    pub fn epoch(&self) -> u64 {
        self.epoch
    }

    pub async fn send(
        &self,
        event: AgentEvent,
    ) -> Result<(), mpsc::error::SendError<(u64, AgentEvent)>> {
        self.inner.send((self.epoch, event)).await
    }

    pub fn try_send(
        &self,
        event: AgentEvent,
    ) -> Result<(), mpsc::error::TrySendError<(u64, AgentEvent)>> {
        self.inner.try_send((self.epoch, event))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyModifiers};

    #[test]
    fn tui_event_key_variant() {
        let key = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE);
        let event = TuiEvent::Key(key);
        assert!(matches!(event, TuiEvent::Key(_)));
    }

    #[test]
    fn tui_event_resize_variant() {
        let event = TuiEvent::Resize {
            width: 120,
            height: 40,
        };
        if let TuiEvent::Resize { width, height } = event {
            assert_eq!(width, 120);
            assert_eq!(height, 40);
        } else {
            panic!("expected Resize");
        }
    }

    #[test]
    fn tui_event_agent_variant() {
        let agent = AgentEvent::ContentDelta {
            index: 0,
            delta: "hello".into(),
        };
        let event = TuiEvent::Agent {
            epoch: 0,
            event: agent,
        };
        assert!(matches!(event, TuiEvent::Agent { .. }));
    }

    #[test]
    fn tui_event_tick_variant() {
        let event = TuiEvent::Tick;
        assert!(matches!(event, TuiEvent::Tick));
    }

    #[test]
    fn tui_event_is_send() {
        fn assert_send<T: Send>() {}
        assert_send::<TuiEvent>();
    }

    #[tokio::test]
    async fn spawn_event_loop_receives_agent_events() {
        let (agent_tx, agent_rx) = mpsc::unbounded_channel::<(u64, AgentEvent)>();
        let broker = Arc::new(EventBroker::new());
        let mut tui_rx = spawn_event_loop(agent_rx, std::time::Duration::from_secs(60), broker);

        agent_tx
            .send((
                42,
                AgentEvent::ContentDelta {
                    index: 0,
                    delta: "test".into(),
                },
            ))
            .unwrap();

        // Should receive the agent event
        let event = tokio::time::timeout(std::time::Duration::from_millis(500), tui_rx.recv())
            .await
            .expect("timeout");

        // Could be a Tick or Agent event — keep consuming until we get Agent
        let mut found_agent = matches!(
            event,
            Some(TuiEvent::Agent {
                epoch: 42,
                event: AgentEvent::ContentDelta { .. },
            })
        );
        if !found_agent {
            for _ in 0..10 {
                if let Ok(Some(e)) =
                    tokio::time::timeout(std::time::Duration::from_millis(100), tui_rx.recv()).await
                    && matches!(
                        e,
                        TuiEvent::Agent {
                            epoch: 42,
                            event: AgentEvent::ContentDelta { .. }
                        }
                    )
                {
                    found_agent = true;
                    break;
                }
            }
        }
        assert!(found_agent, "expected to receive an Agent event");
    }

    #[tokio::test]
    async fn epoch_tagged_sender_attaches_epoch() {
        let (tx, mut rx) = mpsc::channel::<(u64, AgentEvent)>(8);
        let sender = EpochTaggedSender::new(tx, 7);
        sender
            .send(AgentEvent::ContentDelta {
                index: 0,
                delta: "x".into(),
            })
            .await
            .unwrap();
        let (epoch, _ev) = rx.recv().await.expect("expected event");
        assert_eq!(epoch, 7);
    }

    #[tokio::test]
    async fn spawn_event_loop_ticks_continue_when_broker_paused() {
        // The crossterm reader should drop terminal events when paused, but the
        // tick task is independent and must keep firing so the app can render.
        let (_agent_tx, agent_rx) = mpsc::unbounded_channel::<(u64, AgentEvent)>();
        let broker = Arc::new(EventBroker::new());
        broker.pause();
        let mut tui_rx = spawn_event_loop(
            agent_rx,
            std::time::Duration::from_millis(20),
            Arc::clone(&broker),
        );

        let mut found_tick = false;
        for _ in 0..20 {
            if let Ok(Some(e)) =
                tokio::time::timeout(std::time::Duration::from_millis(100), tui_rx.recv()).await
                && matches!(e, TuiEvent::Tick)
            {
                found_tick = true;
                break;
            }
        }
        assert!(found_tick, "ticks must still fire while broker paused");
    }

    #[tokio::test]
    async fn spawn_event_loop_receives_ticks() {
        let (_agent_tx, agent_rx) = mpsc::unbounded_channel::<(u64, AgentEvent)>();
        let broker = Arc::new(EventBroker::new());
        let mut tui_rx = spawn_event_loop(agent_rx, std::time::Duration::from_millis(50), broker);

        // Wait for a tick
        let mut found_tick = false;
        for _ in 0..20 {
            if let Ok(Some(e)) =
                tokio::time::timeout(std::time::Duration::from_millis(100), tui_rx.recv()).await
                && matches!(e, TuiEvent::Tick)
            {
                found_tick = true;
                break;
            }
        }
        assert!(found_tick, "expected to receive a Tick event");
    }
}
