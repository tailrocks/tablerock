use tablerock_core::{
    CommandBudget, CommandBudgetLimits, CommandEnvelope, CommandIntent, CommandScope, ContextId,
    Engine, IdParts, OperationId, OperationScope, PageIdentity, ProfileId, RequestId, ResultId,
    Revision, ServiceCoordinator, ServiceLimits, SessionId,
};
use tablerock_engine::{DriverRuntime, EngineService};

fn opaque<T>(
    low: u64,
    build: impl FnOnce(IdParts) -> Result<T, tablerock_core::IdDecodeError>,
) -> T {
    build(IdParts::new(0, low).unwrap()).unwrap()
}

pub fn operation(low: u64) -> OperationId {
    opaque(low, OperationId::from_parts)
}

pub fn identity(engine: Engine, low: u64) -> PageIdentity {
    PageIdentity::new(opaque(low, ResultId::from_parts), Revision::INITIAL, engine)
}

fn scope() -> OperationScope {
    OperationScope::new(
        opaque(20, ProfileId::from_parts),
        opaque(21, SessionId::from_parts),
        opaque(22, ContextId::from_parts),
    )
}

pub fn service(max_operations: u32, event_capacity: usize) -> EngineService {
    let scope = scope();
    let mut core = ServiceCoordinator::new(ServiceLimits::new(8, max_operations, 2, 8).unwrap());
    core.register_scope(CommandScope::Profile(scope.profile_id()), Revision::INITIAL)
        .unwrap();
    core.register_scope(
        CommandScope::Session {
            profile_id: scope.profile_id(),
            session_id: scope.session_id(),
        },
        Revision::INITIAL,
    )
    .unwrap();
    core.register_scope(CommandScope::Context(scope), Revision::INITIAL)
        .unwrap();
    EngineService::new(
        core,
        DriverRuntime::new(max_operations as usize, event_capacity).unwrap(),
        8,
    )
    .unwrap()
}

pub fn command(request_low: u64) -> CommandEnvelope {
    CommandEnvelope::new(
        opaque(request_low, RequestId::from_parts),
        CommandScope::Context(scope()),
        Revision::INITIAL,
        CommandBudget::new(10_000, 128, 1_048_576, 1024)
            .unwrap()
            .validate(CommandBudgetLimits::new(10_000, 128, 1_048_576, 1024).unwrap())
            .unwrap(),
        None,
        CommandIntent::RefreshCatalog,
    )
    .unwrap()
}
