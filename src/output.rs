use super::FixedPoint;

pub struct Output {
    /// The client the text is regarding
    client: u16,
    /// the amount available for usage, should equal total - held
    available: FixedPoint,
    /// the amount locked during a dispute, should be the total - available
    held: FixedPoint,
    /// the total amount of funds
    total: FixedPoint,
    /// if the account is currently locked due to an ongoing chargeback
    locked: bool,
}

const EXAMPLE_OUTPUT: &str = "client, available, held, total, locked
1, 1.5, 0.0, 1.5, false
2, 2.0, 0.0, 2.0, false";
