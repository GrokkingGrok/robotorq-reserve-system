use serde::{Serialize, Deserialize};
use crate::{TokenId, InvariantError, TokenError};
use crate::hashing::hash_struct;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Token {
    pub id: TokenId,
    pub joule_count: u32,           // operational status
    pub hash: [u8; 32],
}

impl Token {
    pub fn map(joule_count: u32) -> Result<Self, InvariantError> {
        if joule_count == 0 {
            return Err(InvariantError::from(TokenError::ZeroJoules(joule_count)));
        }
        let provisional = Self {
            id: TokenId::new(),
            joule_count,
            hash: [0u8;32],
        };
        let hash = hash_struct(&provisional);
        Ok(Self { hash, ..provisional })
    }
}