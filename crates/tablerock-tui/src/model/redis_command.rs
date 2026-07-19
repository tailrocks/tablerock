//! TUI re-exports the shared Redis command contract owned by the engine.

pub use tablerock_core::{
    RedisCommandLine, RedisCommandPlan, RedisCommandPlanError, RedisCommandSafety,
    classify_redis_command as classify_command, complete_redis_command_prefix as complete_prefix,
    parse_redis_command_line as parse_command_line, plan_redis_command_text as plan_command_text,
    tokenize_redis_command as tokenize,
};
