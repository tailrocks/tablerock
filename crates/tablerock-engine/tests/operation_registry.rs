use tablerock_core::{Engine, IdParts, OperationId};
use tablerock_engine::{
    AdapterError, CancelDispatch, DriverFuture, DriverOperationRegistry, DriverPageRequest,
    DriverPageStream, DriverSession, OperationCancelOutcome, OperationRegistrationError,
};

fn operation(value: u64) -> OperationId {
    OperationId::from_parts(IdParts::new(0, value).unwrap()).unwrap()
}

struct UnsupportedSession;

impl DriverSession for UnsupportedSession {
    fn engine(&self) -> Engine {
        Engine::Redis
    }

    fn start_page_stream<'a>(
        &'a self,
        _request: DriverPageRequest,
    ) -> DriverFuture<'a, Result<Box<dyn DriverPageStream>, AdapterError>> {
        unreachable!()
    }

    fn cancel<'a>(&'a self, _operation_id: OperationId) -> DriverFuture<'a, CancelDispatch> {
        Box::pin(async { CancelDispatch::Unsupported })
    }

    fn shutdown(self: Box<Self>) -> DriverFuture<'static, Result<(), AdapterError>> {
        Box::pin(async { Ok(()) })
    }
}

#[tokio::test]
async fn bounds_identity_and_preserves_cancellation_truth() {
    let first = operation(1);
    let second = operation(2);
    let mut registry = DriverOperationRegistry::new(1);

    registry
        .register(first, Box::new(UnsupportedSession))
        .unwrap();
    assert_eq!(
        registry.register(first, Box::new(UnsupportedSession)),
        Err(OperationRegistrationError::DuplicateOperation)
    );
    assert_eq!(
        registry.register(second, Box::new(UnsupportedSession)),
        Err(OperationRegistrationError::CapacityExhausted)
    );
    assert_eq!(
        registry.cancel(second).await,
        OperationCancelOutcome::UnknownOperation
    );
    assert_eq!(
        registry.cancel(first).await,
        OperationCancelOutcome::Unsupported
    );

    let session = registry.remove(first).unwrap();
    assert!(registry.is_empty());
    session.shutdown().await.unwrap();
}
