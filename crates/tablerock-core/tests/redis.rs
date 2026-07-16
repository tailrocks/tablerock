use tablerock_core::RedisTimeToLive;

#[test]
fn redis_ttl_keeps_missing_persistent_and_expiring_states_distinct() {
    assert!(!RedisTimeToLive::Missing.key_existed_at_observation());
    assert_eq!(RedisTimeToLive::Missing.remaining_millis(), None);

    assert!(RedisTimeToLive::Persistent.key_existed_at_observation());
    assert_eq!(RedisTimeToLive::Persistent.remaining_millis(), None);

    let expiring = RedisTimeToLive::Expiring {
        remaining_millis: 1_500,
    };
    assert!(expiring.key_existed_at_observation());
    assert_eq!(expiring.remaining_millis(), Some(1_500));
}
