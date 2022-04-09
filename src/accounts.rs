use std::collections::{btree_map, BTreeMap, HashMap};
use std::sync::Arc;
use tokio::sync::mpsc::{channel, Sender};
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

use crate::{
    input::{self, Input, TransactionType},
    FixedPoint,
};

type AccountPlace = Account;
// type Accounts = BTreeMap<u16, AccountPlace>;
type Accounts = HashMap<u16, AccountHandler>;

enum Action {
    Input(input::Input),
    Close,
}

pub struct AccountHandler {
    action_sender: Sender<Action>,
    handle: JoinHandle<Account>,
}

impl AccountHandler {
    pub fn new(tx_path: String) -> Self {
        let (action_sender, mut rx) = channel(100);

        let handle = tokio::task::spawn(async move {
            let mut account = Account::new();
            while let Some(action) = rx.recv().await {
                match action {
                    Action::Input(input) => {
                        let _e = account.handle_transaction(input).await;
                    }
                    Action::Close => {
                        break;
                    }
                };
            }
            account
        });

        Self {
            action_sender,
            handle: handle,
        }
    }
    pub async fn handle_transaction(&self, input: input::Input) {
        let input = Action::Input(input);
        let _e = self.action_sender.send(input).await;
    }
    pub async fn close(self) -> Account {
        self.action_sender.send(Action::Close).await;
        self.handle.await.unwrap()
    }
}

pub struct AccountStorage {
    /// We use this to let the code start an own "connection" to the
    /// "database" and search through the history if needed to handle disputes
    tx_path: String,
    accounts: Accounts,
}

impl AccountStorage {
    pub fn new(tx_path: String) -> Self {
        Self {
            tx_path,
            accounts: Default::default(),
        }
    }

    /// Get a reference to the account storage's accounts.
    pub fn accounts(&self) -> &Accounts {
        &self.accounts
    }
    pub fn into_accounts(self) -> Accounts {
        self.accounts
    }

    pub fn get(&mut self, client: u16) -> &mut AccountHandler {
        self.accounts
            .entry(client)
            .or_insert(AccountHandler::new(self.tx_path.clone()))
    }
}

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
    tx_history_db: BTreeMap<u32, Input>,

    /// disputes
    disputes: BTreeMap<u32, (Input, DisputeState)>,
}

impl Account {
    /// Generates a new empty Account
    pub fn new() -> Self {
        Account {
            available: FixedPoint::from_f64(0.0),
            held: FixedPoint::from_f64(0.0),
            locked: false,
            disputes: BTreeMap::new(),

            tx_history_db: Default::default(),
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

    pub async fn handle_transaction(&mut self, transaction: Input) -> Result<(), TransactionError> {
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
                self.tx_history_db
                    .insert(transaction.tx(), transaction.clone());
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
                self.dispute(transaction.tx()).await
            }
            TransactionType::Resolve => {
                // We shall unlock the held funds, if the held funds exist ofcourse
                // If the held funds are already spent, for example by a withdrawal, then a dispute
                self.resolve(transaction.tx()).await
            }
            TransactionType::Chargeback => self.chargeback(transaction.tx()).await,
        };

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

    async fn chargeback(&mut self, tx: u32) -> Result<(), TransactionError> {
        let (input, dispute) = self
            .disputes
            .get_mut(&tx)
            .ok_or(TransactionError::MissingDisputeTx)?;

        println!("checking dispute state input {:?}", input);
        if *dispute == DisputeState::Started {
            println!("dispute has started");
            if let Some(amount) = input.amount_as_fp() {
                println!("the tx in question has an amount");
                if self.held <= amount {
                    println!("the held amount covers the dispute reimbursement");
                    self.held -= amount;
                }
            }
            *dispute = DisputeState::Reimbursed;
            self.lock();
            Ok(())
        } else {
            Err(TransactionError::DisputeAlreadyHandled)
        }
    }

    async fn resolve(&mut self, tx: u32) -> Result<(), TransactionError> {
        // fetch the the tx under dispute, apply the reverse if state is disputed
        let (input, dispute) = self
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

    async fn dispute(&mut self, tx: u32) -> Result<(), TransactionError> {
        // Fetch the tx that is to be disputed
        let input = self
            .search_for_tx(tx)
            .await
            .ok_or(TransactionError::MissingTxId)?;

        match input.r#type() {
            TransactionType::Deposit => {
                println!("{:?}", input);
                if let None = self.disputes.get(&tx) {
                    Err(TransactionError::DisputeAlreadyExist)
                } else {
                    if let Some(amount) = input.amount_as_fp() {
                        if self.available() >= amount {
                            println!("disputed");
                            self.disputes.insert(tx, (input, DisputeState::new()));
                            self.available -= amount;
                            self.held += amount;
                            Ok(())
                        } else {
                            println!("insufficient amount for dispute");
                            Err(TransactionError::NotEnoughAvailableFunds)
                        }
                    } else {
                        println!("huh");
                        Err(TransactionError::InvalidTx)
                    }
                }
            }
            _ => Err(TransactionError::InvalidTxForDispute),
        }
        // store the tx under dispute, unless already handled
        // hold the funds related in the dispute
    }

    // To store on ram, we have to go spelunking in the database after the tx on disputes
    async fn search_for_tx(&self, tx: u32) -> Option<Input> {
        // every entry is an result, we just ignore any faulty parsed input for this case
        let value = self.tx_history_db.get(&tx);

        if let Some(_) = value {
            value.cloned()
        } else {
            //TODO: For future improvements, we would have to look through an external storage of TX
            // let mut csv_reader = input::create_input_deserializer(&self.tx_history_db).await;
            // let csv_iter = csv_reader.deserialize::<input::Input>();
            // use tokio_stream::StreamExt;
            // let mut filter = csv_iter.filter_map(|input| {
            //     if let Ok(input) = input {
            //         if *input.r#type() == TransactionType::Deposit && input.tx() == tx {
            //             return Some(input);
            //         }
            //     }
            //     None
            // });
            // let value = filter.next().await;
            None
        }
    }

    /// Get the account's locked.
    pub fn locked(&self) -> bool {
        self.locked
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn account_deposit_withdraw() {
        let mut account = Account::new();

        let transaction = Input::new(TransactionType::Deposit, 1, 1, Some(55.1234));
        let res = account.handle_transaction(transaction).await;
        if let Err(e) = res {
            assert!(true, "{:?}", e);
        }
        // Withdrawing to much should fail
        assert_eq!(55.1234, account.available());

        // Withdrawing to much should fail
        let transaction = Input::new(TransactionType::Withdrawal, 1, 2, Some(56.1234));
        let res = account.handle_transaction(transaction).await;
        if let Err(e) = res {
            assert!(true, "{:?}", e);
        }
        assert_eq!(55.1234, account.available());

        // Withdrawing a small amount should work
        let transaction = Input::new(TransactionType::Withdrawal, 1, 3, Some(0.1234));
        let res = account.handle_transaction(transaction).await;
        if let Err(e) = res {
            assert!(true, "{:?}", e);
        }
        assert_eq!(55.0, account.available());
        assert_eq!(55.0, account.total());

        // Withdrawing a everything should work
        let transaction = Input::new(TransactionType::Withdrawal, 1, 3, Some(55.0));
        let res = account.handle_transaction(transaction).await;
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
        let res = account.handle_transaction(transaction).await;
        if let Err(e) = res {
            assert!(true, "{:?}", e);
        }

        let transaction = Input::new(TransactionType::Deposit, 1, 2, Some(5.1234));
        let res = account.handle_transaction(transaction).await;
        if let Err(e) = res {
            assert!(true, "{:?}", e);
        }
        assert_eq!(55.1234, account.available());

        // Withdrawing to much should fail
        let transaction = Input::new(TransactionType::Dispute, 1, 1, None);
        let res = account.handle_transaction(transaction).await;
        if let Err(e) = res {
            assert!(true, "{:?}", e);
        }
        assert_eq!(55.1234, account.total());
        assert_eq!(50.0, account.held());
        assert_eq!(5.1234, account.available());

        // Withdrawing a small amount should work, and in this case leave exactly 5.0000 left
        let transaction = Input::new(TransactionType::Withdrawal, 1, 3, Some(0.1234));
        let res = account.handle_transaction(transaction).await;
        if let Err(e) = res {
            assert!(true, "{:?}", e);
        }
        assert_eq!(5.0, account.available());
        assert_eq!(50.0, account.held());
        assert_eq!(55.0, account.total());
    }

    #[tokio::test]
    async fn account_dispute_chargeback() {
        let mut account = Account::new();

        let deposit = Input::new(TransactionType::Deposit, 1, 1, Some(50.0));
        let res = account.handle_transaction(deposit).await;
        if let Err(e) = res {
            assert!(true, "{:?}", e);
        }

        let dispute = Input::new(TransactionType::Dispute, 1, 1, None);
        let res = account.handle_transaction(dispute).await;
        if let Err(e) = res {
            assert!(true, "{:?}", e);
        }
        assert_eq!(0.0, account.available());
        assert_eq!(50.0, account.held());
        assert_eq!(50.0, account.total());
        assert_eq!(false, account.locked(), "account locked state was wrong");

        let chargeback = Input::new(TransactionType::Chargeback, 1, 1, None);
        let res = account.handle_transaction(chargeback).await;
        if let Err(e) = res {
            assert!(true, "{:?}", e);
        }
        assert_eq!(0.0, account.held(), "held amount was wrong");
        assert_eq!(0.0, account.available(), "available amount was wrong");
        assert_eq!(0.0, account.total(), "total amount was wrong");
        assert_eq!(true, account.locked(), "account locked state was wrong");
    }
}
