use std::{
    error::Error,
    fs::{File, OpenOptions},
    io::BufReader,
    path::Path,
    thread,
    time::Duration
};

use chrono::prelude::*;
use csv::Writer;
use env_logger::{Builder, Target, WriteStyle};
use ftx::{
    options::{Endpoint, Options},
    rest::{GetFuture, GetOrderBook, Rest},
};
use log::{debug, info, LevelFilter, trace, warn};
use rust_decimal::prelude::ToPrimitive;
use serde::{Deserialize, Serialize};
use serde_json;
use ta::{indicators::BollingerBands, Next};

#[derive(Serialize, Deserialize)]
struct SettingsFile {
    market_name: String,
    time_delta: u64,
    bb_period: usize,
    bb_std_dev: f64,
    orderbook_depth: u32
}

fn write_to_csv( _utc_time: String, _price: String, _position: String) -> Result<(), Box<dyn Error>> {
    /* Write utc time, price and position to a csv file */
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .append(true)
        .open("positions.csv")
        .unwrap();
    let mut wtr = Writer::from_writer(file);

    wtr.write_record(&[_utc_time, _price, _position])?;
    wtr.flush()?;
    Ok(())
}

#[tokio::main]
async fn main() {
    // Load configuration file
    let settings_filepath = Path::new("settings.json");
    let settings_file = File::open(settings_filepath).expect("Config file not found");
    let reader = BufReader::new(settings_file);
    let settings: SettingsFile =
        serde_json::from_reader(reader).expect("Error when reading config json");

    let mut builder= Builder::new();
    builder
        .filter(None, LevelFilter::Info)
        .write_style(WriteStyle::Always)
        .target(Target::Stdout)
        .init();

    info!("Settings file loaded from {:?}.", settings_filepath);
    info!(
        "market_name={:?}, time_delta={:?}, bb_period={:?}, bb_std_dev={:?}, orderbook_depth={:?}",
        String::from(&settings.market_name),
        settings.time_delta,
        settings.bb_period,
        settings.bb_std_dev,
        settings.orderbook_depth
    );
    info!("Setting trigger in {:?} iterations (approx {:?}s)...",
        settings.bb_period, settings.bb_period * settings.time_delta.to_usize().unwrap());

    // Set up connection to FTX API
    let api = Rest::new(
        Options { endpoint: Endpoint::Com, ..Default::default() }
    );

    // Set up bollinger bands
    let mut bb = BollingerBands::new(settings.bb_period, settings.bb_std_dev).unwrap();

    let mut count: usize = 0;

    loop {
        count += 1;
        let order_book = api.request(
            GetOrderBook { market_name: String::from(&settings.market_name), depth: Option::from(settings.orderbook_depth) }
        ).await;

        let order_book = match order_book {
            Err(e) => {
                warn!("Error: {:?}", e);
                continue
            },
            Ok(o) => o
        };

        let perp_delta = (order_book.bids[0].1 - order_book.asks[0].1).to_f64().unwrap();

        let out = bb.next(perp_delta);
        let bb_lower = out.lower;
        let bb_upper = out.upper;

        debug!("perp_delta={:.2}, bb_lower={:.2}, bb_upper={:.2}", perp_delta, bb_lower, bb_upper);

        if count > settings.bb_period {
            if count == settings.bb_period + 1 {
                warn!("Trigger is now set...")
            }

            if perp_delta > bb_upper || perp_delta < bb_lower {
                let btc_price = api.request(
                    GetFuture { future_name: String::from(&settings.market_name) }
                ).await;


                let btc_price = match btc_price {
                    Err(e) => {
                        warn!("Error: {:?}", e);
                        continue
                    },
                    Ok(o) => o
                };

                let mut price: f64 = 0.0;
                let mut position: String = "none".to_string();

                if perp_delta > bb_upper {
                    price = btc_price.ask.unwrap().to_f64().unwrap();
                    position = "short".to_string();
                    warn!("Perp delta above upper bb, going {} at {:.2}", position, price);
                } else if perp_delta < bb_lower {
                    price = btc_price.bid.unwrap().to_f64().unwrap();
                    position = "long".to_string();
                    warn!("Perp delta below lower bb, going {} at {:.2}", position, price);
                }
                // Write the positions to a csv
                let utc_time: DateTime<Utc> = Utc::now();
                write_to_csv(utc_time.to_string(), price.to_string(), position);
            }
        }
        thread::sleep(Duration::from_secs(settings.time_delta));
    }
}
