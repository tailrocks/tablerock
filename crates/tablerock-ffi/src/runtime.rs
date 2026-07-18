use std::sync::{Arc, Mutex};

use tokio::runtime::Runtime;

use crate::error::BridgeError;

/// Process-owned multi-thread Tokio runtime for the UniFFI facade.
///
/// Construction and destruction are explicit and idempotent: a second
/// `ensure` reuses the live runtime; a second `shutdown` is a no-op success.
///
/// `block_on` is nest-safe: when the calling thread is already inside a Tokio
/// runtime (e.g. `#[tokio::test]`), work is dispatched onto a dedicated OS
/// thread that owns `block_on` for the facade runtime.
pub(crate) struct RuntimeOwner {
    runtime: Mutex<Option<Arc<Runtime>>>,
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
                .map_err(|error| BridgeError::rejected("runtime-build", error.to_string()))?;
            *guard = Some(Arc::new(runtime));
        }
        Ok(())
    }

    pub(crate) fn block_on<T>(&self, future: impl std::future::Future<Output = T> + Send) -> Result<T, BridgeError>
    where
        T: Send,
    {
        let runtime = {
            let guard = self
                .runtime
                .lock()
                .map_err(|_| BridgeError::rejected("runtime-lock", "runtime mutex poisoned"))?;
            Arc::clone(guard.as_ref().ok_or(BridgeError::RuntimeUnavailable)?)
        };
        if tokio::runtime::Handle::try_current().is_ok() {
            // Nested runtime: cannot call block_on on this thread.
            std::thread::scope(|scope| {
                scope
                    .spawn(|| runtime.block_on(future))
                    .join()
                    .map_err(|_| BridgeError::rejected("runtime-join", "bridge worker panicked"))
            })
        } else {
            Ok(runtime.block_on(future))
        }
    }

    /// Drops the runtime if present. Idempotent.
    pub(crate) fn shutdown(&self) -> Result<(), BridgeError> {
        let mut guard = self
            .runtime
            .lock()
            .map_err(|_| BridgeError::rejected("runtime-lock", "runtime mutex poisoned"))?;
        if let Some(runtime) = guard.take() {
            // Drop our Arc; if tests/callers hold no clones, runtime ends.
            // Use try_unwrap when unique; otherwise leave background drain.
            match Arc::try_unwrap(runtime) {
                Ok(runtime) => runtime.shutdown_background(),
                Err(shared) => drop(shared),
            }
        }
        Ok(())
    }
}
