use anyhow::Result;

/// Represents the "Clover" Consistency Check (from Verified Software research).
///
/// Any artifact that claims to implement another artifact must satisfy this trait.
/// For example, a `Spec` must be consistent with a `Requirement`.
/// A `Code` block must be consistent with a `Spec`.
pub trait ConsistencyCheck<Target: ?Sized> {
    /// Checks if `self` is consistent with `target`.
    /// Returns `Ok(())` if consistent, or an error description if strictly inconsistent.
    ///
    /// Note: In a Neuro-Symbolic system, this implementation might call out to an LLM
    /// or a formal verifier to perform the check.
    fn check_consistency(&self, target: &Target) -> Result<()>;
}

/// Represents a verifiable artifact that has an Oracle.
pub trait Verifiable {
    /// Returns true if the artifact passes its internal verification logic
    /// (e.g., tests pass, linter passes).
    fn verify(&self) -> Result<bool>;
}
