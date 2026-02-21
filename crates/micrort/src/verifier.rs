//! Bytecode verifier — StackMapTable verification and type safety checks.

use rava_common::error::Result;

/// Verifies Java bytecode before interpretation.
///
/// Required for security: dynamic class loading means arbitrary bytecode can
/// arrive at runtime. The verifier prevents malformed or malicious bytecode
/// from corrupting the runtime.
///
/// Implements a subset of the JVM spec §4.10 (type checking verifier).
pub struct BytecodeVerifier;

impl BytecodeVerifier {
    pub fn new() -> Self {
        Self
    }

    /// Verify a class's bytecode. Returns `Ok(())` if the bytecode is valid.
    pub fn verify(&self, _bytecode: &[u8]) -> Result<()> {
        // TODO(phase-3): implement StackMapTable verification
        Err(rava_common::error::RavaError::Other("verifier not yet implemented".into()))
    }
}

impl Default for BytecodeVerifier {
    fn default() -> Self {
        Self::new()
    }
}
