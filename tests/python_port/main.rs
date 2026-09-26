//! Ports of the former Python suite (EA-19), built as ONE test binary.
//!
//! One module per retired `tests/test_*.py` file. New tests go here, in Rust;
//! do not add Python tests.
#![allow(missing_docs)]

mod campaign_measurement_plan;
mod common;
mod corpus_reproduction;
mod migration_contract;
mod module;
mod onboarding_script;
mod workflows;
