//! Gas metering for operations.

#[cfg(test)]
mod tests;

/// Unit used to measure gas.
pub type Gas = u64;

/// A mapping from an operation to its gas cost.
pub trait OpGasCost<Op> {
    /// The gas cost associated with the given op.
    fn op_gas_cost(&self, op: &Op) -> Gas;
}

impl<F, Op> OpGasCost<Op> for F
where
    F: Fn(&Op) -> Gas,
{
    fn op_gas_cost(&self, op: &Op) -> Gas {
        (*self)(op)
    }
}
