//! Process-local session registry: one owned connection, many borrowed ops.

use std::{collections::BTreeMap, error::Error, fmt, sync::Arc};

use tablerock_core::{CancelDispatch, Engine, OperationId, SessionId};
use tokio::sync::RwLock;

use crate::{
    AdapterError, AdapterFailureClass, CatalogRequest, CatalogSubtree, DriverFuture,
    DriverPageRequest, DriverPageStream, DriverSession, LocalForwardTunnel, ServerDescribe,
    SessionHealth,
};

/// Upper bound for concurrent registered sessions (ServiceLimits scale).
pub const MAX_REGISTERED_SESSIONS: usize = 1_024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionRegistryError {
    InvalidLimits,
    CapacityExceeded,
    DuplicateSession,
    UnknownSession,
    SessionBusy,
    Shutdown(AdapterError),
}

impl fmt::Display for SessionRegistryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidLimits => "session registry limits are invalid",
            Self::CapacityExceeded => "session registry capacity exceeded",
            Self::DuplicateSession => "session id is already registered",
            Self::UnknownSession => "session id is not registered",
            Self::SessionBusy => "session still has active operation borrows",
            Self::Shutdown(_) => "session shutdown failed",
        })
    }
}

impl Error for SessionRegistryError {}

enum SessionState {
    Open(Box<dyn DriverSession>),
    Closed,
}

/// Shared session slot. Operations borrow `Arc` clones; disconnect shuts down
/// only when the registry holds the last reference.
///
/// Optional SSH local-forward tunnel is owned here so it outlives the driver
/// connection and drops (closing bastion channels) on session shutdown.
pub struct SessionSlot {
    engine: Engine,
    state: RwLock<SessionState>,
    tunnel: std::sync::Mutex<Option<LocalForwardTunnel>>,
}

impl SessionSlot {
    fn with_tunnel(session: Box<dyn DriverSession>, tunnel: Option<LocalForwardTunnel>) -> Self {
        Self {
            engine: session.engine(),
            state: RwLock::new(SessionState::Open(session)),
            tunnel: std::sync::Mutex::new(tunnel),
        }
    }

    async fn shutdown_exclusive(&self) -> Result<(), AdapterError> {
        let mut guard = self.state.write().await;
        let result = match std::mem::replace(&mut *guard, SessionState::Closed) {
            SessionState::Open(session) => session.shutdown().await,
            SessionState::Closed => Ok(()),
        };
        if let Ok(mut tunnel) = self.tunnel.lock() {
            tunnel.take();
        }
        result
    }
}

impl DriverSession for SessionSlot {
    fn engine(&self) -> Engine {
        self.engine
    }

    fn start_page_stream<'a>(
        &'a self,
        request: DriverPageRequest,
    ) -> DriverFuture<'a, Result<Box<dyn DriverPageStream>, AdapterError>> {
        Box::pin(async move {
            let guard = self.state.read().await;
            match &*guard {
                SessionState::Open(session) => session.start_page_stream(request).await,
                SessionState::Closed => Err(AdapterError::new(
                    self.engine,
                    AdapterFailureClass::Connection,
                )),
            }
        })
    }

    fn cancel<'a>(&'a self, operation_id: OperationId) -> DriverFuture<'a, CancelDispatch> {
        Box::pin(async move {
            let guard = self.state.read().await;
            match &*guard {
                SessionState::Open(session) => session.cancel(operation_id).await,
                SessionState::Closed => CancelDispatch::PreventedBeforeDispatch,
            }
        })
    }

    fn health<'a>(&'a self) -> DriverFuture<'a, Result<SessionHealth, AdapterError>> {
        Box::pin(async move {
            let guard = self.state.read().await;
            match &*guard {
                SessionState::Open(session) => session.health().await,
                SessionState::Closed => Err(AdapterError::new(
                    self.engine,
                    AdapterFailureClass::Connection,
                )),
            }
        })
    }

    fn catalog<'a>(
        &'a self,
        request: CatalogRequest,
    ) -> DriverFuture<'a, Result<CatalogSubtree, AdapterError>> {
        Box::pin(async move {
            let guard = self.state.read().await;
            match &*guard {
                SessionState::Open(session) => session.catalog(request).await,
                SessionState::Closed => Err(AdapterError::new(
                    self.engine,
                    AdapterFailureClass::Connection,
                )),
            }
        })
    }

    fn describe<'a>(&'a self) -> DriverFuture<'a, Result<ServerDescribe, AdapterError>> {
        Box::pin(async move {
            let guard = self.state.read().await;
            match &*guard {
                SessionState::Open(session) => session.describe().await,
                SessionState::Closed => Err(AdapterError::new(
                    self.engine,
                    AdapterFailureClass::Connection,
                )),
            }
        })
    }

    fn apply_authorized_mutation<'a>(
        &'a self,
        authorized: tablerock_core::AuthorizedMutationPlan,
    ) -> DriverFuture<'a, Result<crate::MutationApplyOutcome, AdapterError>> {
        Box::pin(async move {
            let guard = self.state.read().await;
            match &*guard {
                SessionState::Open(session) => session.apply_authorized_mutation(authorized).await,
                SessionState::Closed => Err(AdapterError::new(
                    self.engine,
                    AdapterFailureClass::Connection,
                )),
            }
        })
    }

    fn execute_ddl_plan<'a>(
        &'a self,
        plan: tablerock_core::DdlPlan,
    ) -> DriverFuture<'a, Result<(), AdapterError>> {
        Box::pin(async move {
            let guard = self.state.read().await;
            match &*guard {
                SessionState::Open(session) => session.execute_ddl_plan(plan).await,
                SessionState::Closed => Err(AdapterError::new(
                    self.engine,
                    AdapterFailureClass::Connection,
                )),
            }
        })
    }

    fn redis_key_view_lines<'a>(
        &'a self,
        key: &'a [u8],
    ) -> DriverFuture<'a, Result<(String, Vec<String>), AdapterError>> {
        Box::pin(async move {
            let guard = self.state.read().await;
            match &*guard {
                SessionState::Open(session) => session.redis_key_view_lines(key).await,
                SessionState::Closed => Err(AdapterError::new(
                    self.engine,
                    AdapterFailureClass::Connection,
                )),
            }
        })
    }

    fn redis_info_lines<'a>(
        &'a self,
    ) -> DriverFuture<'a, Result<(u64, Vec<String>), AdapterError>> {
        Box::pin(async move {
            let guard = self.state.read().await;
            match &*guard {
                SessionState::Open(session) => session.redis_info_lines().await,
                SessionState::Closed => Err(AdapterError::new(
                    self.engine,
                    AdapterFailureClass::Connection,
                )),
            }
        })
    }

    fn shutdown(self: Box<Self>) -> DriverFuture<'static, Result<(), AdapterError>> {
        Box::pin(async move { self.shutdown_exclusive().await })
    }
}

pub struct SessionRegistry {
    max_sessions: usize,
    sessions: BTreeMap<SessionId, Arc<SessionSlot>>,
}

impl SessionRegistry {
    pub fn new(max_sessions: usize) -> Result<Self, SessionRegistryError> {
        if max_sessions == 0 || max_sessions > MAX_REGISTERED_SESSIONS {
            return Err(SessionRegistryError::InvalidLimits);
        }
        Ok(Self {
            max_sessions,
            sessions: BTreeMap::new(),
        })
    }

    pub fn register(
        &mut self,
        session_id: SessionId,
        session: Box<dyn DriverSession>,
    ) -> Result<Arc<dyn DriverSession>, SessionRegistryError> {
        self.register_with_tunnel(session_id, session, None)
    }

    /// Register a session that was opened through an SSH local-forward tunnel.
    /// The tunnel is held until the session is disconnected/shutdown.
    pub fn register_with_tunnel(
        &mut self,
        session_id: SessionId,
        session: Box<dyn DriverSession>,
        tunnel: Option<LocalForwardTunnel>,
    ) -> Result<Arc<dyn DriverSession>, SessionRegistryError> {
        if self.sessions.contains_key(&session_id) {
            return Err(SessionRegistryError::DuplicateSession);
        }
        if self.sessions.len() >= self.max_sessions {
            return Err(SessionRegistryError::CapacityExceeded);
        }
        let slot = Arc::new(SessionSlot::with_tunnel(session, tunnel));
        self.sessions.insert(session_id, Arc::clone(&slot));
        Ok(slot as Arc<dyn DriverSession>)
    }

    #[must_use]
    pub fn session(&self, session_id: SessionId) -> Option<Arc<dyn DriverSession>> {
        self.sessions
            .get(&session_id)
            .map(|slot| Arc::clone(slot) as Arc<dyn DriverSession>)
    }

    #[must_use]
    pub fn contains(&self, session_id: SessionId) -> bool {
        self.sessions.contains_key(&session_id)
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.sessions.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.sessions.is_empty()
    }

    /// Remove and shut down the session when no operation still holds a borrow.
    pub async fn disconnect(&mut self, session_id: SessionId) -> Result<(), SessionRegistryError> {
        let Some(slot) = self.sessions.remove(&session_id) else {
            return Err(SessionRegistryError::UnknownSession);
        };
        if Arc::strong_count(&slot) > 1 {
            self.sessions.insert(session_id, slot);
            return Err(SessionRegistryError::SessionBusy);
        }
        // Exclusive: only this Arc remains. Sized try_unwrap works for SessionSlot.
        let slot = match Arc::try_unwrap(slot) {
            Ok(slot) => slot,
            Err(_) => unreachable!("strong_count checked exclusive before try_unwrap"),
        };
        slot.shutdown_exclusive()
            .await
            .map_err(SessionRegistryError::Shutdown)
    }
}
