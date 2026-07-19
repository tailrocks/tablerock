//! Shared Redis command tokenizer, safety classification, and execution planning.
//!
//! Official command names are recorded as a build-time table (provenance:
//! Redis command reference family names — not a vendored third-party dump).
//! Unknown commands classify as writes. Blocking commands are denied for the
//! shared session path.

/// Safety class for a tokenized command line.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RedisCommandSafety {
    ReadOnly,
    MayWrite,
    /// BLPOP/BRPOP/… — must not run on the shared session.
    BlockingDenied,
    Empty,
}

impl RedisCommandSafety {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::ReadOnly => "read-only",
            Self::MayWrite => "may-write",
            Self::BlockingDenied => "blocking-denied",
            Self::Empty => "empty",
        }
    }
}

/// Tokenized command (argv[0] uppercased).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RedisCommandLine {
    pub name: String,
    pub args: Vec<String>,
    pub safety: RedisCommandSafety,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RedisCommandPlan {
    Pipeline(Vec<RedisPlannedCommand>),
    BlockingPop { command: String, key: Vec<u8> },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RedisPlannedCommand {
    pub name: String,
    pub args: Vec<Vec<u8>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RedisCommandPlanError {
    InputTooLarge,
    TooManyCommands,
    ArgumentTooLarge,
    BlockingMustBeAlone,
    BlockingDenied(String),
}

impl std::fmt::Display for RedisCommandPlanError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InputTooLarge => formatter.write_str("Redis command input exceeds 64 KiB"),
            Self::TooManyCommands => formatter.write_str("Redis pipeline exceeds 64 commands"),
            Self::ArgumentTooLarge => formatter.write_str("Redis command argument exceeds 8 KiB"),
            Self::BlockingMustBeAlone => formatter.write_str("blocking BLPOP/BRPOP must be alone"),
            Self::BlockingDenied(command) => {
                write!(formatter, "blocking command denied: {command}")
            }
        }
    }
}

impl std::error::Error for RedisCommandPlanError {}

/// Parse a bounded multiline command editor buffer into one shared execution plan.
pub fn plan_command_text(input: &str) -> Result<Option<RedisCommandPlan>, RedisCommandPlanError> {
    if input.len() > 64 * 1024 {
        return Err(RedisCommandPlanError::InputTooLarge);
    }
    let mut commands = Vec::new();
    let mut blocking: Option<(String, Vec<u8>)> = None;
    for raw in input.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let parsed = parse_command_line(line);
        if parsed.name.len() > 8 * 1024 || parsed.args.iter().any(|arg| arg.len() > 8 * 1024) {
            return Err(RedisCommandPlanError::ArgumentTooLarge);
        }
        match parsed.safety {
            RedisCommandSafety::Empty => {}
            RedisCommandSafety::BlockingDenied => {
                if !matches!(parsed.name.as_str(), "BLPOP" | "BRPOP") || parsed.args.is_empty() {
                    return Err(RedisCommandPlanError::BlockingDenied(parsed.name));
                }
                if blocking.is_some() || !commands.is_empty() {
                    return Err(RedisCommandPlanError::BlockingMustBeAlone);
                }
                blocking = Some((parsed.name, parsed.args[0].as_bytes().to_vec()));
            }
            RedisCommandSafety::ReadOnly | RedisCommandSafety::MayWrite => {
                if blocking.is_some() {
                    return Err(RedisCommandPlanError::BlockingMustBeAlone);
                }
                commands.push(RedisPlannedCommand {
                    name: parsed.name,
                    args: parsed.args.into_iter().map(String::into_bytes).collect(),
                });
                if commands.len() > 64 {
                    return Err(RedisCommandPlanError::TooManyCommands);
                }
            }
        }
    }
    Ok(match blocking {
        Some((command, key)) => Some(RedisCommandPlan::BlockingPop { command, key }),
        None if commands.is_empty() => None,
        None => Some(RedisCommandPlan::Pipeline(commands)),
    })
}

/// Split a command line into argv; respects simple double quotes.
pub fn tokenize(input: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut in_quotes = false;
    for ch in input.chars() {
        match ch {
            '"' if !in_quotes => in_quotes = true,
            '"' if in_quotes => in_quotes = false,
            c if c.is_whitespace() && !in_quotes => {
                if !cur.is_empty() {
                    out.push(std::mem::take(&mut cur));
                }
            }
            c => cur.push(c),
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

/// Classify a tokenized argv[0] (case-insensitive).
#[must_use]
pub fn classify_command(name: &str) -> RedisCommandSafety {
    if name.is_empty() {
        return RedisCommandSafety::Empty;
    }
    let upper = name.to_ascii_uppercase();
    if BLOCKING.contains(&upper.as_str()) {
        return RedisCommandSafety::BlockingDenied;
    }
    if READ_ONLY.contains(&upper.as_str()) {
        return RedisCommandSafety::ReadOnly;
    }
    // Unknown commands are writes (fail closed for safety).
    RedisCommandSafety::MayWrite
}

/// Parse + classify one line.
pub fn parse_command_line(input: &str) -> RedisCommandLine {
    let mut args = tokenize(input);
    if args.is_empty() {
        return RedisCommandLine {
            name: String::new(),
            args: Vec::new(),
            safety: RedisCommandSafety::Empty,
        };
    }
    let name = args.remove(0).to_ascii_uppercase();
    let safety = classify_command(&name);
    RedisCommandLine { name, args, safety }
}

/// Completion candidates matching a prefix (uppercased names).
pub fn complete_prefix(prefix: &str, limit: usize) -> Vec<&'static str> {
    let p = prefix.to_ascii_uppercase();
    ALL_COMMANDS
        .iter()
        .copied()
        .filter(|c| c.starts_with(&p))
        .take(limit.max(1))
        .collect()
}

// Provenance: Redis open-command family names used for classification only.
// Not a full dump of redis-doc; curated subsets for safety gates.
const BLOCKING: &[&str] = &[
    "BLPOP",
    "BRPOP",
    "BRPOPLPUSH",
    "BLMOVE",
    "BZPOPMIN",
    "BZPOPMAX",
    "BZMPOP",
    "BLMPOP",
    "XREAD",
    "XREADGROUP", // can block with BLOCK option — deny on shared path
];

const READ_ONLY: &[&str] = &[
    "GET",
    "MGET",
    "STRLEN",
    "GETRANGE",
    "SUBSTR",
    "EXISTS",
    "TYPE",
    "TTL",
    "PTTL",
    "HGET",
    "HMGET",
    "HGETALL",
    "HKEYS",
    "HVALS",
    "HLEN",
    "HEXISTS",
    "HSCAN",
    "LRANGE",
    "LINDEX",
    "LLEN",
    "SCARD",
    "SISMEMBER",
    "SMEMBERS",
    "SSCAN",
    "SRANDMEMBER",
    "ZRANGE",
    "ZRANGEBYSCORE",
    "ZCARD",
    "ZSCORE",
    "ZRANK",
    "ZSCAN",
    "ZCOUNT",
    "XRANGE",
    "XREVRANGE",
    "XLEN",
    "XINFO",
    "SCAN",
    "DBSIZE",
    "INFO",
    "PING",
    "ECHO",
    "TIME",
    "CLIENT",
    "CONFIG",
    "MEMORY",
    "OBJECT",
    "DUMP",
    "KEYS", // KEYS classified read-only but UI must never issue it for browse
];

/// Curated completion table (read + write + blocking families).
///
/// Provenance decision: Redis open-command *names* only, hand-curated for the
/// first program. Not a redis-doc dump or third-party JSON (license gate).
/// Expand here as product coverage grows; classification lists remain authority
/// for safety.
const ALL_COMMANDS: &[&str] = &[
    "APPEND",
    "BLPOP",
    "BRPOP",
    "COPY",
    "DBSIZE",
    "DECR",
    "DEL",
    "DUMP",
    "ECHO",
    "EXISTS",
    "EXPIRE",
    "GET",
    "GETRANGE",
    "HDEL",
    "HGET",
    "HGETALL",
    "HLEN",
    "HMGET",
    "HSCAN",
    "HSET",
    "INCR",
    "INFO",
    "KEYS",
    "LINDEX",
    "LLEN",
    "LPOP",
    "LPUSH",
    "LRANGE",
    "MGET",
    "MSET",
    "PERSIST",
    "PEXPIRE",
    "PING",
    "PTTL",
    "RENAME",
    "RESTORE",
    "RPOP",
    "RPUSH",
    "SADD",
    "SCAN",
    "SCARD",
    "SET",
    "SETRANGE",
    "SISMEMBER",
    "SMEMBERS",
    "SREM",
    "SSCAN",
    "STRLEN",
    "TTL",
    "TYPE",
    "XADD",
    "XLEN",
    "XRANGE",
    "XREAD",
    "ZADD",
    "ZCARD",
    "ZRANGE",
    "ZREM",
    "ZSCAN",
    "ZSCORE",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokenize_respects_quotes() {
        let t = tokenize(r#"SET key "hello world""#);
        assert_eq!(t, vec!["SET", "key", "hello world"]);
    }

    #[test]
    fn unknown_is_write_blocking_denied() {
        assert_eq!(classify_command("GET"), RedisCommandSafety::ReadOnly);
        assert_eq!(classify_command("SET"), RedisCommandSafety::MayWrite);
        assert_eq!(
            classify_command("MYMODULE.FOO"),
            RedisCommandSafety::MayWrite
        );
        assert_eq!(
            classify_command("BLPOP"),
            RedisCommandSafety::BlockingDenied
        );
        assert_eq!(
            classify_command("XREAD"),
            RedisCommandSafety::BlockingDenied
        );
    }

    #[test]
    fn parse_and_complete() {
        let line = parse_command_line("get mykey");
        assert_eq!(line.name, "GET");
        assert_eq!(line.safety, RedisCommandSafety::ReadOnly);
        let hits = complete_prefix("HGE", 8);
        assert!(hits.contains(&"HGET"));
    }

    #[test]
    fn no_rollback_language_in_labels() {
        for s in [
            RedisCommandSafety::ReadOnly,
            RedisCommandSafety::MayWrite,
            RedisCommandSafety::BlockingDenied,
        ] {
            assert!(!s.label().contains("rollback"));
            assert!(!s.label().contains("transaction"));
        }
    }

    #[test]
    fn classify_empty_and_case_insensitive() {
        // Empty argv classifies as Empty.
        assert_eq!(classify_command(""), RedisCommandSafety::Empty);
        // Classification is case-insensitive (Redis commands are).
        assert_eq!(classify_command("get"), RedisCommandSafety::ReadOnly);
        assert_eq!(
            classify_command("blpop"),
            RedisCommandSafety::BlockingDenied
        );
        // KEYS is a read operation in the command classifier (the auto-browse
        // SCAN-only rule is enforced in the browse path, not here).
        assert_eq!(classify_command("Keys"), RedisCommandSafety::ReadOnly);
    }

    #[test]
    fn shared_planner_bounds_pipeline_and_isolates_blocking_pop() {
        let Some(RedisCommandPlan::Pipeline(commands)) =
            plan_command_text("SET k v\nGET k\n").unwrap()
        else {
            panic!("pipeline expected");
        };
        assert_eq!(commands.len(), 2);
        assert_eq!(commands[0].name, "SET");
        assert_eq!(commands[1].args, vec![b"k".to_vec()]);

        assert_eq!(
            plan_command_text("BLPOP queue 0").unwrap(),
            Some(RedisCommandPlan::BlockingPop {
                command: "BLPOP".into(),
                key: b"queue".to_vec(),
            })
        );
        assert_eq!(
            plan_command_text("BLPOP queue 0\nGET k"),
            Err(RedisCommandPlanError::BlockingMustBeAlone)
        );
        assert!(matches!(
            plan_command_text("XREAD STREAMS a 0"),
            Err(RedisCommandPlanError::BlockingDenied(command)) if command == "XREAD"
        ));
    }
}
