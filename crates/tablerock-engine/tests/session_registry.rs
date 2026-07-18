use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};

use tablerock_core::{
    CancelDispatch, Engine, IdParts, OperationId, PageIdentity, ResultId, Revision, SessionId,
};
use tablerock_engine::{
    AdapterError, DriverFuture, DriverPageRequest, DriverPageStream, DriverSession,
    SessionRegistry, SessionRegistryError,
};

struct CountingSession {
    shutdowns: Arc<AtomicUsize>,
    shutdown_flag: Arc<AtomicBool>,
}

struct EmptyStream;

impl DriverPageStream for EmptyStream {
    fn next_page<'a>(
        &'a mut self,
        _identity: PageIdentity,
        _start_row: u64,
    ) -> DriverFuture<'a, Result<Option<tablerock_core::ResultPage>, AdapterError>> {
        Box::pin(async { Ok(None) })
    }
}

impl DriverSession for CountingSession {
    fn engine(&self) -> Engine {
        Engine::PostgreSql
    }

    fn start_page_stream<'a>(
        &'a self,
        _request: DriverPageRequest,
    ) -> DriverFuture<'a, Result<Box<dyn DriverPageStream>, AdapterError>> {
        Box::pin(async { Ok(Box::new(EmptyStream) as Box<dyn DriverPageStream>) })
    }

    fn cancel<'a>(&'a self, _operation_id: OperationId) -> DriverFuture<'a, CancelDispatch> {
        Box::pin(async { CancelDispatch::Unsupported })
    }

    fn health<'a>(
        &'a self,
    ) -> DriverFuture<'a, Result<tablerock_engine::SessionHealth, AdapterError>> {
        Box::pin(async {
            Ok(tablerock_engine::SessionHealth::new(
                Engine::PostgreSql,
                true,
                0,
            ))
        })
    }

    fn shutdown(self: Box<Self>) -> DriverFuture<'static, Result<(), AdapterError>> {
        Box::pin(async move {
            self.shutdowns.fetch_add(1, Ordering::SeqCst);
            self.shutdown_flag.store(true, Ordering::SeqCst);
            Ok(())
        })
    }
}

fn session_id(low: u64) -> SessionId {
    SessionId::from_parts(IdParts::new(0, low).unwrap()).unwrap()
}

fn counting() -> (Box<dyn DriverSession>, Arc<AtomicUsize>, Arc<AtomicBool>) {
    let shutdowns = Arc::new(AtomicUsize::new(0));
    let shutdown_flag = Arc::new(AtomicBool::new(false));
    (
        Box::new(CountingSession {
            shutdowns: Arc::clone(&shutdowns),
            shutdown_flag: Arc::clone(&shutdown_flag),
        }),
        shutdowns,
        shutdown_flag,
    )
}

#[tokio::test]
async fn register_lookup_and_exclusive_disconnect() {
    let mut registry = SessionRegistry::new(2).unwrap();
    let id = session_id(1);
    let (session, shutdowns, flag) = counting();
    let handle = registry.register(id, session).unwrap();
    assert!(registry.contains(id));
    assert!(registry.session(id).is_some());
    assert_eq!(registry.len(), 1);

    // Borrow keeps disconnect busy.
    assert_eq!(
        registry.disconnect(id).await,
        Err(SessionRegistryError::SessionBusy)
    );
    drop(handle);
    registry.disconnect(id).await.unwrap();
    assert!(!registry.contains(id));
    assert!(flag.load(Ordering::SeqCst));
    assert_eq!(shutdowns.load(Ordering::SeqCst), 1);

    // Second disconnect is unknown.
    assert_eq!(
        registry.disconnect(id).await,
        Err(SessionRegistryError::UnknownSession)
    );
}

#[tokio::test]
async fn capacity_and_duplicate_fail_closed() {
    let mut registry = SessionRegistry::new(1).unwrap();
    let first = session_id(1);
    let second = session_id(2);
    let (session, _, _) = counting();
    registry.register(first, session).unwrap();
    let (session, _, _) = counting();
    assert!(matches!(
        registry.register(first, session),
        Err(SessionRegistryError::DuplicateSession)
    ));
    let (session, _, _) = counting();
    assert!(matches!(
        registry.register(second, session),
        Err(SessionRegistryError::CapacityExceeded)
    ));
    assert!(matches!(
        SessionRegistry::new(0),
        Err(SessionRegistryError::InvalidLimits)
    ));
}

#[tokio::test]
async fn disconnect_shuts_down_exactly_once() {
    let mut registry = SessionRegistry::new(4).unwrap();
    let id = session_id(9);
    let (session, shutdowns, _) = counting();
    let a = registry.register(id, session).unwrap();
    let b = registry.session(id).unwrap();
    drop(a);
    drop(b);
    registry.disconnect(id).await.unwrap();
    assert_eq!(shutdowns.load(Ordering::SeqCst), 1);
}

// Silence unused import when tests focus on registry only.
#[allow(dead_code)]
fn _page_identity() -> PageIdentity {
    PageIdentity::new(
        ResultId::from_parts(IdParts::new(0, 1).unwrap()).unwrap(),
        Revision::INITIAL,
        Engine::PostgreSql,
    )
}
