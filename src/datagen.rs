// Just to create testfiles that one can benchmark against
mod accounts;
mod input;
mod simple_fp;

use input::{Input, TransactionType};
use simple_fp::FixedPoint;
use std::fs::OpenOptions;
use std::io::prelude::*;

use rand::Rng;

fn main() {
    let filename = std::env::args()
        .into_iter()
        .nth(1)
        .expect("Expected file name as argument");

    let mut i: u32 = 0;
    let mut input = input::Input::new(TransactionType::Deposit, 0, 0, Some(1.0));

    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .open(filename)
        .unwrap();

    let mut s = String::new();

    let _e = s.push_str(&format!("type, client, tx, amount\n"));

    while i < u16::MAX as u32 * 10 {
        let _e = s.push_str(&format!(
            "{}, {}, {}, {:0.4}\n",
            input.r#type(),
            input.client(),
            input.tx(),
            input.amount().unwrap()
        ));
        let mut rng = rand::thread_rng();
        let deposit: f32 = rng.gen_range(0.0..100.0);

        let txtype = if deposit > 10.0 {
            TransactionType::Deposit
        } else {
            TransactionType::Withdrawal
        };

        input = Input::new(
            txtype,
            input.client().checked_add(1).map_or(0, |v| v),
            i,
            input.amount(),
        );

        if i % 100 == 0 {
            let dispute = Input::new(TransactionType::Dispute, input.client(), i, None);
            let _e = s.push_str(&format!(
                "{}, {}, {}, ,\n",
                dispute.r#type(),
                dispute.client(),
                dispute.tx(),
            ));

            let mut rng = rand::thread_rng();
            let resolve: f32 = rng.gen_range(0.0..100.0);

            if resolve > 10.0 {
                let dispute = Input::new(TransactionType::Resolve, input.client(), i, None);
                let _e = s.push_str(&format!(
                    "{}, {}, {}, ,\n",
                    dispute.r#type(),
                    dispute.client(),
                    dispute.tx(),
                ));
            } else {
                let dispute = Input::new(TransactionType::Chargeback, input.client(), i, None);
                let _e = s.push_str(&format!(
                    "{}, {}, {}, ,\n",
                    dispute.r#type(),
                    dispute.client(),
                    dispute.tx(),
                ));
            }
        }

        i += 1;

        if s.len() > 1000000 {
            let _e = write!(file, "{}", s);
            s = String::new();
        }
    }
    if s.len() > 0 {
        let _e = write!(file, "{}", s);
    }
}
