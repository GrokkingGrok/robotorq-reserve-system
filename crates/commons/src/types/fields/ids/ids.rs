use serde::{Serialize, Deserialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RobotId(pub Uuid);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TokenId(pub Uuid);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct UnmappedOreBatchId(pub Uuid);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TripleTorqId(pub Uuid);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ContractId(pub Uuid);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PartyId(pub Uuid);

impl RobotId { pub fn new() -> Self { Self(Uuid::new_v4()) } }
impl TokenId { pub fn new() -> Self { Self(Uuid::new_v4()) } }
impl UnmappedOreBatchId { pub fn new() -> Self { Self(Uuid::new_v4()) } }
impl TripleTorqId { pub fn new() -> Self { Self(Uuid::new_v4()) } }
impl ContractId { pub fn new() -> Self { Self(Uuid::new_v4()) } }
impl PartyId { pub fn new() -> Self { Self(Uuid::new_v4()) } }
