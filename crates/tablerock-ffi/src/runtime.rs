use std::sync::Mutex;

use tokio::runtime::Runtime;

use crate::error::BridgeError;

/// Process-owned multi-thread Tokio runtime for the UniFFI facade.
///
/// Construction and destruction are explicit and idempotent: a second
/// `ensure` reuses the live runtime; a second `shutdown` is a no-op success.
pub(crate) struct RuntimeOwner {
    runtime: Mutex<Option<Runtime>>,
}

impl RuntimeOwner {
    pub(crate) fn new() -> Self {
        Self {
            runtime: Mutex::new(None),
        }
    }

    pub(crate) fn ensure(&self) -> Result<(), BridgeError> {
        let mut guard = self
            .runtime
            .lock()
            .map_err(|_| BridgeError::rejected("runtime-lock", "runtime mutex poisoned"))?;
        if guard.is_none() {
            let runtime = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .thread_name("tablerock-bridge")
                .worker_threads(2)
                .build()
                .map_err(|error| {
                    BridgeError::rejected("runtime-build", error.to_string())
                })?;
            *guard = Some(runtime);
        }
        Ok(())
    }

    pub(crate) fn block_on<T>(&self, future: impl std::future::Future<Output = T>) -> Result<T, BridgeError> {
        let guard = self
            .runtime
            .lock()
            .map_err(|_| BridgeError::rejected("runtime-lock", "runtime mutex poisoned"))?;
        let runtime = guard.as_ref().ok_or(BridgeError::RuntimeUnavailable)?;
        Ok(runtime.block_on(future))
    }

    /// Drops the runtime if present. Idempotent.
    pub(crate) fn shutdown(&self) -> Result<(), BridgeError> {
        let mut guard = self
            .runtime
            .lock()
            .map_err(|_| BridgeError::rejected("runtime-lock", "runtime mutex poisoned"))?;
        if let Some(runtime) = guard.take() {
            runtime.shutdown_background();
        }
        Ok(())
    }

}
