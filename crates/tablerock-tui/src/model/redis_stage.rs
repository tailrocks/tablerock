//! Parse operator paste buffers into Redis collection mutation specs.

use crate::effect::MutationChangeSpec;

/// Safe field/member token: 1..=512 of printable non-whitespace (no `=` for bare tokens).
fn is_safe_token(s: &str) -> bool {
    !s.is_empty() && s.len() <= 512 && s.chars().all(|c| !c.is_control() && !c.is_whitespace())
}

/// Parse stage buffer for a Redis collection op into a presentation spec.
///
/// Formats:
/// - `hset`: `field=value` (first `=`)
/// - `zadd`: `score=member` (first `=`) or `score member`
/// - `hdel` / `sadd` / `srem` / `zrem`: bare token
#[must_use]
pub fn parse_stage_buffer(op: &str, buffer: &str) -> Option<MutationChangeSpec> {
    let buf = buffer.trim();
    if buf.is_empty() {
        return None;
    }
    match op {
        "hset" => {
            let (field, value) = buf.split_once('=')?;
            let field = field.trim();
            let value = value.trim();
            if !is_safe_token(field) || value.is_empty() || value.len() > 65_536 {
                return None;
            }
            Some(MutationChangeSpec::RedisHashSet {
                field: field.to_owned(),
                value: value.to_owned(),
            })
        }
        "hdel" => {
            if !is_safe_token(buf) {
                return None;
            }
            Some(MutationChangeSpec::RedisHashDelete {
                field: buf.to_owned(),
            })
        }
        "sadd" => {
            if !is_safe_token(buf) {
                return None;
            }
            Some(MutationChangeSpec::RedisSetAdd {
                member: buf.to_owned(),
            })
        }
        "srem" => {
            if !is_safe_token(buf) {
                return None;
            }
            Some(MutationChangeSpec::RedisSetRemove {
                member: buf.to_owned(),
            })
        }
        "zadd" => {
            let (score, member) = if let Some((s, m)) = buf.split_once('=') {
                (s.trim(), m.trim())
            } else {
                let mut parts = buf.split_whitespace();
                let s = parts.next()?;
                let m = parts.next()?;
                if parts.next().is_some() {
                    return None;
                }
                (s, m)
            };
            if !is_safe_token(member) {
                return None;
            }
            let n: f64 = score.parse().ok()?;
            if !n.is_finite() {
                return None;
            }
            Some(MutationChangeSpec::RedisZSetAdd {
                member: member.to_owned(),
                score: score.to_owned(),
            })
        }
        "zrem" => {
            if !is_safe_token(buf) {
                return None;
            }
            Some(MutationChangeSpec::RedisZSetRemove {
                member: buf.to_owned(),
            })
        }
        _ => None,
    }
}

/// Map inspector kind label + add/remove to op id.
#[must_use]
pub fn op_for_kind(kind_label: &str, add: bool) -> Option<&'static str> {
    match (kind_label.to_ascii_lowercase().as_str(), add) {
        ("hash", true) => Some("hset"),
        ("hash", false) => Some("hdel"),
        ("set", true) => Some("sadd"),
        ("set", false) => Some("srem"),
        ("zset" | "sortedset" | "sorted_set", true) => Some("zadd"),
        ("zset" | "sortedset" | "sorted_set", false) => Some("zrem"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_hset_and_zadd() {
        assert!(matches!(
            parse_stage_buffer("hset", "f=v"),
            Some(MutationChangeSpec::RedisHashSet { field, value })
                if field == "f" && value == "v"
        ));
        assert!(matches!(
            parse_stage_buffer("zadd", "2.5=m"),
            Some(MutationChangeSpec::RedisZSetAdd { member, score })
                if member == "m" && score == "2.5"
        ));
        assert!(matches!(
            parse_stage_buffer("zadd", "1.0 mem"),
            Some(MutationChangeSpec::RedisZSetAdd { member, score })
                if member == "mem" && score == "1.0"
        ));
    }

    #[test]
    fn reject_hostile_and_nonfinite() {
        assert!(parse_stage_buffer("hset", "").is_none());
        assert!(parse_stage_buffer("hset", "nofield").is_none());
        assert!(parse_stage_buffer("sadd", "has space").is_none());
        assert!(parse_stage_buffer("zadd", "nan=m").is_none());
        assert!(parse_stage_buffer("zadd", "inf=m").is_none());
        assert!(parse_stage_buffer("hdel", "").is_none());
    }

    #[test]
    fn op_for_kind_maps_hash_set_zset() {
        assert_eq!(op_for_kind("hash", true), Some("hset"));
        assert_eq!(op_for_kind("SET", false), Some("srem"));
        assert_eq!(op_for_kind("zset", true), Some("zadd"));
        assert_eq!(op_for_kind("string", true), None);
    }
}
