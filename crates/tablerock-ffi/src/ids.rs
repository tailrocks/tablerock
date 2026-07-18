use tablerock_core::{
    CatalogNodeId, ContextId, IdDecodeError, IdParts, OperationId, ProfileId, ResultId,
    ReviewTokenId, SessionId,
};

pub(crate) fn catalog_node_from_bytes(bytes: &[u8]) -> Result<CatalogNodeId, IdDecodeError> {
    CatalogNodeId::from_bytes(as_array16(bytes)?)
}

pub(crate) fn catalog_node_bytes(id: CatalogNodeId) -> Vec<u8> {
    id.to_bytes().to_vec()
}

pub(crate) fn session_from_bytes(bytes: &[u8]) -> Result<SessionId, IdDecodeError> {
    SessionId::from_bytes(as_array16(bytes)?)
}

pub(crate) fn operation_from_bytes(bytes: &[u8]) -> Result<OperationId, IdDecodeError> {
    OperationId::from_bytes(as_array16(bytes)?)
}

pub(crate) fn result_from_bytes(bytes: &[u8]) -> Result<ResultId, IdDecodeError> {
    ResultId::from_bytes(as_array16(bytes)?)
}

pub(crate) fn review_token_from_bytes(bytes: &[u8]) -> Result<ReviewTokenId, IdDecodeError> {
    ReviewTokenId::from_bytes(as_array16(bytes)?)
}

pub(crate) fn session_bytes(id: SessionId) -> Vec<u8> {
    id.to_bytes().to_vec()
}

pub(crate) fn operation_bytes(id: OperationId) -> Vec<u8> {
    id.to_bytes().to_vec()
}

pub(crate) fn review_token_bytes(id: ReviewTokenId) -> Vec<u8> {
    id.to_bytes().to_vec()
}

fn as_array16(bytes: &[u8]) -> Result<[u8; 16], IdDecodeError> {
    <[u8; 16]>::try_from(bytes).map_err(|_| IdDecodeError::InvalidLength)
}

/// Sequential opaque ID factory for bridge-owned handles.
pub(crate) struct IdFactory {
    high: u64,
    next_low: u64,
}

impl IdFactory {
    pub(crate) fn new() -> Self {
        let high = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_nanos() as u64)
            .unwrap_or(1)
            .max(1);
        Self { high, next_low: 1 }
    }

    pub(crate) fn parts(&mut self) -> IdParts {
        let low = self.next_low;
        self.next_low = self.next_low.saturating_add(1);
        IdParts::new(self.high, low).expect("factory high is nonzero")
    }

    pub(crate) fn profile(&mut self) -> ProfileId {
        ProfileId::from_parts(self.parts()).expect("nonzero id")
    }

    pub(crate) fn session(&mut self) -> SessionId {
        SessionId::from_parts(self.parts()).expect("nonzero id")
    }

    pub(crate) fn context(&mut self) -> ContextId {
        ContextId::from_parts(self.parts()).expect("nonzero id")
    }

    pub(crate) fn operation(&mut self) -> OperationId {
        OperationId::from_parts(self.parts()).expect("nonzero id")
    }

    pub(crate) fn result(&mut self) -> ResultId {
        ResultId::from_parts(self.parts()).expect("nonzero id")
    }

    pub(crate) fn review_token(&mut self) -> ReviewTokenId {
        ReviewTokenId::from_parts(self.parts()).expect("nonzero id")
    }

    pub(crate) fn mutation(&mut self) -> tablerock_core::MutationId {
        tablerock_core::MutationId::from_parts(self.parts()).expect("nonzero id")
    }

    pub(crate) fn catalog_node(&mut self) -> CatalogNodeId {
        CatalogNodeId::from_parts(self.parts()).expect("nonzero id")
    }
}
