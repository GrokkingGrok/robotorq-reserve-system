// Re-export Triple decomposition utilities from common crate.
pub use common::triples::{
    Triple,
    INGOTS_PER_ROBOTORQ,
    ORE_PER_INGOT,
    JOULETORQ_PER_ROBOTORQ,
    jouletorq_to_triple,
    triple_to_jouletorq,
    add_triples,
    subtract_triples,
    triple_to_decimal,
    decimal_to_triple,
};