use std::collections::BTreeMap;

use crate::{
    input::{Input, TransactionType},
    FixedPoint,
};

pub type AccountStorage<'a> = BTreeMap<u16, Account>;

#[derive(Debug)]
pub enum TransactionError {
    /// There was not enough funds on the account to  handle the requested transaction
    NotEnoughAvailableFunds,
    /// The Transaction ID could not be found
    MissingTxId,
    /// Account has been locked, and thus no transaction should be valid
    AccountLocked,
    /// The transaction was not valid for some reason
    InvalidTx,
    /// The transactio ID to dispute was invalid for some reason
    InvalidTxForDispute,
    /// The TxId for the dispute was missing
    MissingDisputeTx,
    /// The Dispute has already been started
    DisputeAlreadyExist,
    /// The Dispute has already been resolved one way or another
    DisputeAlreadyHandled,
}

#[derive(PartialEq, Eq)]
pub enum DisputeState {
    Started,
    Reimbursed,
    Resolved,
}

impl DisputeState {
    fn new() -> Self {
        Self::Started
    }
}

pub struct Account {
    /// amount of usable funds for withdrawal, trading, etc
    available: FixedPoint,
    /// amount of held funds for dispute
    held: FixedPoint,
    /// is the account locked or not
    locked: bool,

    /// Just store an entire history of each transaction performed
    tx_history: BTreeMap<u32, Input>,

    /// disputes
    disputes: BTreeMap<u32, DisputeState>,
}

impl<'a> Account {
    /// Generates a new empty Account
    pub fn new() -> Self {
        Account {
            available: FixedPoint::from_f64(0.0),
            held: FixedPoint::from_f64(0.0),
            locked: false,
            disputes: BTreeMap::new(),
            tx_history: BTreeMap::new(),
        }
    }
    /// available
    pub fn available(&self) -> FixedPoint {
        self.available
    }

    /// Get the account's held.
    pub fn held(&self) -> FixedPoint {
        self.held
    }
    pub fn total(&self) -> FixedPoint {
        self.held + self.available
    }

    fn lock(&mut self) {
        self.locked = true;
    }

    pub fn handle_transaction(&mut self, transaction: Input) -> Result<(), TransactionError> {
        if !transaction.valid() {
            return Err(TransactionError::InvalidTx);
        }
        if self.locked {
            // This is probably a much more complex case, since an account probably can have multiple
            // active disputes. But I also feel like trying to handle this without careful consideration
            // could be quite exploitable, which is unwanted. So I'll play it safe here, and just not handle more transactions
            // after a chargeback has occured
            return Err(TransactionError::AccountLocked);
        }

        let tx_res = match transaction.r#type() {
            TransactionType::Deposit => {
                // Safe because of the validity check on the transaction
                let amount = transaction.amount_as_fp().unwrap();
                self.deposit(amount);
                Ok(())
            }
            TransactionType::Withdrawal => {
                // Safe because of the validity check on the transaction
                let amount = transaction.amount_as_fp().unwrap();
                self.withdraw(amount)
            }
            TransactionType::Dispute => {
                // we need to look back into all of the history related to this client ( and this client only ),
                // to validate wheter the TX exists, and then we need to hold the amount found in that tx
                self.dispute(transaction.tx())
            }
            TransactionType::Resolve => {
                // We shall unlock the held funds, if the held funds exist ofcourse
                // If the held funds are already spent, for example by a withdrawal, then a dispute
                self.resolve(transaction.tx())
            }
            TransactionType::Chargeback => self.chargeback(transaction.tx()),
        };

        if let Ok(_) = tx_res {
            self.tx_history.insert(transaction.tx(), transaction);
        }
        tx_res
    }

    fn deposit(&mut self, amount: FixedPoint) {
        self.available += amount;
    }

    fn withdraw(&mut self, amount: FixedPoint) -> Result<(), TransactionError> {
        if self.available >= amount {
            self.available -= amount;
            Ok(())
        } else {
            Err(TransactionError::NotEnoughAvailableFunds)
        }
    }

    fn chargeback(&mut self, tx: u32) -> Result<(), TransactionError> {
        let input = self
            .search_for_tx(tx)
            .ok_or(TransactionError::MissingTxId)?;

        let dispute = self
            .disputes
            .get_mut(&tx)
            .ok_or(TransactionError::MissingDisputeTx)?;

        if *dispute == DisputeState::Started {
            if let Some(amount) = input.amount_as_fp() {
                self.held -= amount;
            }
            *dispute = DisputeState::Reimbursed;
            self.lock();
            Ok(())
        } else {
            Err(TransactionError::DisputeAlreadyHandled)
        }
    }

    fn resolve(&mut self, tx: u32) -> Result<(), TransactionError> {
        let input = self
            .search_for_tx(tx)
            .ok_or(TransactionError::MissingTxId)?;
        // fetch the the tx under dispute, apply the reverse if state is disputed
        let dispute = self
            .disputes
            .get_mut(&tx)
            .ok_or(TransactionError::MissingDisputeTx)?;

        if *dispute == DisputeState::Started {
            if let Some(amount) = input.amount_as_fp() {
                let heldres = self.held - amount;
                if heldres < FixedPoint::from_f64(0.0) {
                    eprintln!(
                        "resolved a dispute resulting in negative held amount for TX: {}",
                        tx
                    );
                }
                self.held = heldres;
                self.available += amount;
                *dispute = DisputeState::Resolved;
                Ok(())
            } else {
                Err(TransactionError::InvalidTx)
            }
        } else {
            Err(TransactionError::DisputeAlreadyHandled)
        }
    }

    fn dispute(&mut self, tx: u32) -> Result<(), TransactionError> {
        // Fetch the tx that is to be disputed
        let input = self
            .search_for_tx(tx)
            .ok_or(TransactionError::MissingTxId)?;

        match input.r#type() {
            TransactionType::Deposit => {
                if self.disputes.contains_key(&tx) {
                    Err(TransactionError::DisputeAlreadyExist)
                } else {
                    if let Some(amount) = input.amount_as_fp() {
                        if self.available() >= amount {
                            self.disputes.insert(tx, DisputeState::new());
                            self.available -= amount;
                            self.held += amount;
                            Ok(())
                        } else {
                            Err(TransactionError::NotEnoughAvailableFunds)
                        }
                    } else {
                        Err(TransactionError::InvalidTx)
                    }
                }
            }
            _ => Err(TransactionError::InvalidTxForDispute),
        }
        // store the tx under dispute, unless already handled
        // hold the funds related in the dispute
    }

    fn search_for_tx(&self, tx: u32) -> Option<Input> {
        let local = self.tx_history.get(&tx);

        if let Some(_) = local {
            local.cloned()
        } else {
            //TODO: For future improvements, we would have to look through an external storage of TX
            None
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn account_deposit_withdraw() {
        let mut account = Account::new();

        let transaction = Input::new(TransactionType::Deposit, 1, 1, Some(55.1234));
        let res = account.handle_transaction(transaction);
        if let Err(e) = res {
            assert!(true, "{:?}", e);
        }
        // Withdrawing to much should fail
        assert_eq!(55.1234, account.available());

        // Withdrawing to much should fail
        let transaction = Input::new(TransactionType::Withdrawal, 1, 2, Some(56.1234));
        let res = account.handle_transaction(transaction);
        if let Err(e) = res {
            assert!(true, "{:?}", e);
        }
        assert_eq!(55.1234, account.available());

        // Withdrawing a small amount should work
        let transaction = Input::new(TransactionType::Withdrawal, 1, 3, Some(0.1234));
        let res = account.handle_transaction(transaction);
        if let Err(e) = res {
            assert!(true, "{:?}", e);
        }
        assert_eq!(55.0, account.available());
        assert_eq!(55.0, account.total());

        // Withdrawing a everything should work
        let transaction = Input::new(TransactionType::Withdrawal, 1, 3, Some(55.0));
        let res = account.handle_transaction(transaction);
        if let Err(e) = res {
            assert!(true, "{:?}", e);
        }
        assert_eq!(0.0, account.available());
        assert_eq!(0.0, account.total());
    }

    #[tokio::test]
    async fn account_deposited_dispute() {
        let mut account = Account::new();

        let transaction = Input::new(TransactionType::Deposit, 1, 1, Some(50.0));
        let res = account.handle_transaction(transaction);
        if let Err(e) = res {
            assert!(true, "{:?}", e);
        }

        let transaction = Input::new(TransactionType::Deposit, 1, 2, Some(5.1234));
        let res = account.handle_transaction(transaction);
        if let Err(e) = res {
            assert!(true, "{:?}", e);
        }
        // Withdrawing to much should fail
        assert_eq!(55.1234, account.available());

        // Withdrawing to much should fail
        let transaction = Input::new(TransactionType::Dispute, 1, 1, None);
        let res = account.handle_transaction(transaction);
        if let Err(e) = res {
            assert!(true, "{:?}", e);
        }
        assert_eq!(55.1234, account.total());
        assert_eq!(5.1234, account.available());
        assert_eq!(50.0, account.held());

        // Withdrawing a small amount should work, and in this case leave exactly 5.0000 left
        let transaction = Input::new(TransactionType::Withdrawal, 1, 3, Some(0.1234));
        let res = account.handle_transaction(transaction);
        if let Err(e) = res {
            assert!(true, "{:?}", e);
        }
        assert_eq!(5.0, account.available());
        assert_eq!(50.0, account.held());
        assert_eq!(55.0, account.total());
    }
}
