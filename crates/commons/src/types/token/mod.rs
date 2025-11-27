use serde::{Serialize, Deserialize};
use crate::types::ids::TokenId;
use crate::util::error::InvariantError;
use crate::util::error::token_error::TokenError;
use crate::util::hashing::hash_struct;
use crate::util::schema::TOKEN_SCHEMA_VERSION;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Token {
    pub id: TokenId,
    pub joule_count: u32,           // operational status
    pub schema_version: u32,
    pub hash: [u8; 32],
}

impl Token {
    /// Canonical constructor for a Token; prefer this over `map`.
    pub fn new(joule_count: u32) -> Result<Self, InvariantError> {
        Self::map(joule_count)
    }

    pub fn map(joule_count: u32) -> Result<Self, InvariantError> {
        if joule_count == 0 {
            return Err(InvariantError::from(TokenError::ZeroJoules(joule_count)));
        }
        let provisional = Self {
            id: TokenId::new(),
            joule_count,
            schema_version: TOKEN_SCHEMA_VERSION,
            hash: [0u8;32],
        };
        let hash = hash_struct(&provisional);
        Ok(Self { hash, ..provisional })
    }
}