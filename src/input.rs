use std::fmt::Display;

use crate::FixedPoint;

use csv_async::AsyncDeserializer;
use serde::Deserialize;
use tokio::fs::File;

#[derive(Debug, Deserialize, Clone)]
pub struct Input {
    /// This is the type of the input, it can only be a fixed amount of values
    r#type: TransactionType,

    /// client ID number
    client: u16,

    tx: u32,
    /// These are fixed point numbers, but we will treat them as f64 for simple serialization and deserialization
    amount: Option<f64>,
}

impl Input {
    /// The input can be wrong, since the optional items in the input, actually has some logic to them that has to be checked
    pub fn valid(&self) -> bool {
        match self.r#type {
            TransactionType::Deposit | TransactionType::Withdrawal => {
                // We dont allow negative values, since that is basically what the type is declaring
                if let Some(amount) = self.amount {
                    if amount > 0.0 {
                        true
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            TransactionType::Dispute | TransactionType::Resolve | TransactionType::Chargeback => {
                self.amount.is_none()
            }
        }
    }

    /// Get the input's client.
    pub fn client(&self) -> u16 {
        self.client
    }

    /// Get a reference to the input's r#type.
    pub fn r#type(&self) -> &TransactionType {
        &self.r#type
    }

    /// Get the input's amount
    pub fn amount_as_fp(&self) -> Option<FixedPoint> {
        self.amount.map(|v| FixedPoint::from_f64(v))
    }

    /// Get the input's tx.
    pub fn tx(&self) -> u32 {
        self.tx
    }

    /// only to create easier test transactions
    pub fn new(r#type: TransactionType, client: u16, tx: u32, amount: Option<f64>) -> Self {
        Self {
            r#type,
            client,
            tx,
            amount,
        }
    }

    /// Get the input's amount.
    pub fn amount(&self) -> Option<f64> {
        self.amount
    }
}

pub async fn create_input_deserializer(pathname: &str) -> AsyncDeserializer<tokio::fs::File> {
    let file = File::open(pathname).await.unwrap();

    let rdr = csv_async::AsyncReaderBuilder::new()
        .delimiter(b',')
        .trim(csv_async::Trim::All)
        .flexible(true)
        .create_deserializer(file);
    rdr
}

#[derive(Copy, Clone, Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TransactionType {
    Deposit,
    Withdrawal,
    Dispute,
    Resolve,
    Chargeback,
}

impl Display for TransactionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            TransactionType::Deposit => "deposit",
            TransactionType::Withdrawal => "withdrawal",
            TransactionType::Dispute => "dispute",
            TransactionType::Resolve => "resolve",
            TransactionType::Chargeback => "chargeback",
        };
        f.write_str(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio_stream::StreamExt;

    #[tokio::test]
    async fn parsing_input_works() {
        let mut rdr = create_input_deserializer("testdata/input.csv").await;

        let amount: Vec<Input> = rdr
            .deserialize()
            // just crash on errors in input for this test
            .map(|e: Result<Input, _>| e.unwrap())
            .filter(|tx| tx.valid())
            .collect()
            .await;

        assert_eq!(8, amount.len());
    }
}
