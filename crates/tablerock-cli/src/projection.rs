//! Map core/persistence facts into TUI presentation projections.

use tablerock_core::{Engine, ProfileListItem};
use tablerock_tui::ProfileRowProjection;

#[must_use]
pub fn profile_row(item: &ProfileListItem) -> ProfileRowProjection {
    ProfileRowProjection {
        name: item.name().as_str().to_owned(),
        engine_label: engine_label(item.engine()).to_owned(),
        group: item.group().map(|group| group.as_str().to_owned()),
        favorite: item.favorite(),
    }
}

const fn engine_label(engine: Engine) -> &'static str {
    match engine {
        Engine::PostgreSql => "PostgreSQL",
        Engine::ClickHouse => "ClickHouse",
        Engine::Redis => "Redis",
    }
}
