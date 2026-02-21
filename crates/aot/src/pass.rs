//! Optimization pass trait.

use rava_common::error::Result;
use rava_rir::Module;

/// A single optimization pass that transforms a RIR module in place.
pub trait OptPass: Send + Sync {
    fn name(&self) -> &'static str;
    fn run(&self, module: &mut Module) -> Result<()>;
}
