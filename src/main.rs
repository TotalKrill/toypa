use tokio_stream::StreamExt;

mod accounts;
mod input;
mod output;
mod simple_fp;

use simple_fp::FixedPoint;
// mod transaction;

#[tokio::main]
async fn main() {
    let filename = std::env::args()
        .into_iter()
        .nth(1)
        .expect("Expected file name as argument");

    let mut csv_reader = input::create_input_deserializer(&filename).await;

    let mut accounts = accounts::AccountStorage::new(&filename);

    let mut csv_iter = csv_reader.deserialize::<input::Input>();

    while let Some(csv_res) = csv_iter.next().await {
        // every entry is an result, we just ignore any faulty parsed input for this case
        if let Ok(input) = csv_res {
            // then check the accountstorage for an existing account,
            // if none exists, we should create one,
            // and then try to apply the transaction to that account if valid,
            if input.valid() {
                let entry = accounts
                    .entry(input.client())
                    .or_insert(accounts::Account::new());

                // We could check how the transaction went, if we wanted to
                let _res = entry.handle_transaction(input);
            }
        }
    }

    output::print_from_accounts(accounts);
}
