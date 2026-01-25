#[cfg(test)]
use ifcl_core::*;
use chrono::Utc;
use uuid::Uuid;

#[tokio::test]
async fn test_session_state_parity_after_replay() {
    let store = SqliteEventStore::new("sqlite::memory:").await.unwrap();
    let session_id = Uuid::new_v4();

    // 1. Create some events
    let events = vec![
        Event {
            id: Uuid::new_v4(),
            session_id,
            trace_id: Uuid::new_v4(),
            timestamp: Utc::now(),
            worker_id: "system".to_string(),
            event_type: "LoopStarted".to_string(),
            payload: serde_json::to_string(&LoopConfig {
                goal: "Test Goal".to_string(),
                max_coins: Some(100),
            })
            .unwrap(),
        },
        Event {
            id: Uuid::new_v4(),
            session_id,
            trace_id: Uuid::new_v4(),
            timestamp: Utc::now(),
            worker_id: "system".to_string(),
            event_type: "WorkerJoined".to_string(),
            payload: serde_json::to_string(&WorkerProfile {
                name: "TestWorker".to_string(),
                role: WorkerRole::Coder,
                model: None,
            })
            .unwrap(),
        },
    ];

    for e in events {
        store.append(e).await.unwrap();
    }

    // 2. Replay events into a "mock state" (simulating TUI replay)
    let replayed_events = store.list(session_id).await.unwrap();

    let mut workers = Vec::new();
    let mut goal = String::new();

    for event in replayed_events {
        match event.event_type.as_str() {
            "WorkerJoined" => {
                if let Ok(profile) = serde_json::from_str::<WorkerProfile>(&event.payload) {
                    workers.push(profile);
                }
            }
            "LoopStarted" => {
                if let Ok(config) = serde_json::from_str::<LoopConfig>(&event.payload) {
                    goal = config.goal;
                }
            }
            _ => {}
        }
    }

    // 3. Verify parity
    assert_eq!(goal, "Test Goal");
    assert_eq!(workers.len(), 1);
    assert_eq!(workers[0].name, "TestWorker");
}
