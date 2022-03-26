use ftx::{
    options::{Endpoint, Options},
    rest::{GetOrderBook, Rest, Result},
};
use rust_decimal::prelude::ToPrimitive;
use std::{thread, time::Duration};
use ta::indicators::{BollingerBands};
use ta::Next;


#[tokio::main]
async fn main() -> Result<()> {
    // Number of data points to collect
    const NUM_POINTS: usize = 3;
    // Time gap in seconds between points
    const TIME_DELTA: u64 = 2;
    // Period and standard deviation of bollinger bands
    const BB_PERIOD: usize = 3;
    const BB_STD_DEV: f64 = 2.0;

    // Set up connection to FTX API
    let api = Rest::new(
        Options { endpoint: Endpoint::Com, ..Default::default() }
    );

    // Set up empty data arrays
    let mut perp_deltas: [f64; NUM_POINTS] = [0.0; NUM_POINTS];
    let mut bb_lower: [f64; NUM_POINTS] = [0.0; NUM_POINTS];
    let mut bb_upper: [f64; NUM_POINTS] = [0.0; NUM_POINTS];

    let mut bb = BollingerBands::new(BB_PERIOD, BB_STD_DEV).unwrap();

    for i in 0..NUM_POINTS {
        let order_book = api.request(
            GetOrderBook { market_name: "BTC-PERP".to_string(), depth: 1.to_u32() }
        ).await?;
        perp_deltas[i] = (order_book.bids[0].1 - order_book.asks[0].1).to_f64().unwrap();

        thread::sleep(Duration::from_secs(TIME_DELTA));
        let out = bb.next(perp_deltas[i]);
        bb_lower[i] = out.lower;
        bb_upper[i] = out.upper;
    }

    println!("{:?}", perp_deltas);
    println!("{:?}", bb_lower);
    println!("{:?}", bb_upper);
    println!("{:?}", perp_deltas[perp_deltas.len() - 1]);

    if perp_deltas[NUM_POINTS - 1] > bb_upper[NUM_POINTS - 1] {
        println!("Perp delta ({}) above upper bb ({}), going short",
                 perp_deltas[NUM_POINTS - 1], bb_upper[NUM_POINTS - 1])
    }
    else if perp_deltas[NUM_POINTS - 1] < bb_lower[NUM_POINTS - 1] {
        println!("Perp delta ({}) below lower bb ({}), going long",
                 perp_deltas[NUM_POINTS - 1], bb_lower[NUM_POINTS - 1])
    }

    Ok(())
}
