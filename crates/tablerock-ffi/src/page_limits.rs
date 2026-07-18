use tablerock_core::PageLimits;

/// Default page limits for the native bridge (matches engine probe budgets).
#[must_use]
pub fn default_page_limits() -> PageLimits {
    PageLimits::new(500, 64, 4 * 1024 * 1024, 64 * 1024)
}
