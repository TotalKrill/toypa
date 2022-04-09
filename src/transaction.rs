use crate::{input::Input, FixedPoint};

/// the different types of transactions that can occur
pub enum Transaction {
    Deposit(Deposit),
    Withdrawal(Withdrawal),
    Dispute(Dispute),
    Resolve(Resolve),
    Chargeback(Chargeback),
}
impl TryFrom<Input> for Transaction {
    fn try_from(input: Input) -> Self {}
}

/// A chargeback transaction request
pub struct Chargeback {
    client: u16,
    tx: u32,
}

/// Marking a tx as resolved
pub struct Resolve {
    client: u16,
    tx: u32,
}

/// Marking a tx as disputed, for a certain client
pub struct Dispute {
    client: u16,
    tx: u32,
}

pub struct Withdrawal {
    client: u16,
    tx: u32,
    amount: FixedPoint,
}

pub struct Deposit {
    client: u16,
    tx: u32,
    amount: FixedPoint,
}
