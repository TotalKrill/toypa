use crate::accounts;

pub struct Output {
    /// The client the text is regarding
    client: u16,
    /// the amount available for usage, should equal total - held
    available: f64,
    /// the amount locked during a dispute, should be the total - available
    held: f64,
    /// the total amount of funds
    total: f64,
    /// if the account is currently locked due to an ongoing chargeback
    locked: bool,
}

impl Output {
    pub fn csv_line(&self) -> String {
        format!(
            "{}, {:0.4}, {:0.4}, {:0.4}, {}",
            self.client, self.available, self.held, self.total, self.locked
        )
    }
}

pub fn print_from_accounts(accountstore: accounts::AccountStorage) -> () {
    // using csv writer for this, just seems uneccesary...
    // especially since no formatting rules are really in effect

    println!("client, available, held, total, locked");

    for (client, account) in accountstore.accounts() {
        let out = Output {
            client: *client,
            available: account.available().to_f64(),
            held: account.held().to_f64(),
            total: account.total().to_f64(),
            locked: account.locked(),
        };
        let s = out.csv_line();
        println!("{}", s);
    }
}
