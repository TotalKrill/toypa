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
    let csv_iter = csv_reader.deserialize::<input::Input>();

    let mut accounts = accounts::AccountStorage::new(filename);

    let mut filter = csv_iter.filter_map(|item| {
        if let Ok(input) = item {
            if input.valid() {
                return Some(input);
            }
        }
        None
    });

    while let Some(input) = filter.next().await {
        let entry = accounts.get(input.client());

        // We could check how the transaction went, if we wanted to
        let _e = entry.handle_transaction(input).await;
    }

    let mut map = std::collections::HashMap::new();
    for (client, account) in accounts.into_accounts().drain() {
        let account = account.close().await;
        map.insert(client.clone(), account);
    }

    output::print_from_accounts_map(map);
}
