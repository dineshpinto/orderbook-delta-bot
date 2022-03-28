use rust_decimal::prelude::ToPrimitive;

mod helpers;

#[tokio::main]
async fn main() {
    /// Main logical loop, checks entry conditions
    /// and enters trade if conditions are favorable

    // Load configuration file
    let settings_filepath = std::path::Path::new("settings.json");
    let settings_file = std::fs::File::open(settings_filepath)
        .expect("Config file not found");
    let reader = std::io::BufReader::new(settings_file);
    let settings: helpers::SettingsFile =
        serde_json::from_reader(reader).expect("Error when reading config json");

    let mut builder = env_logger::Builder::new();
    builder
        .filter(None, log::LevelFilter::Info)
        .write_style(env_logger::WriteStyle::Always)
        .target(env_logger::Target::Stdout)
        .init();

    log::info!("Settings file loaded from {:?}.", settings_filepath);
    log::info!(
        "market_name={:?}, time_delta={:?}, bb_period={:?}, bb_std_dev={:?}, orderbook_depth={:?}, \
        positions_filename={:?}",
        String::from(&settings.market_name),
        settings.time_delta,
        settings.bb_period,
        settings.bb_std_dev,
        settings.orderbook_depth,
        settings.positions_filename
    );
    if settings.live {
        log::warn!("The bot is running live")
    }
    log::info!("Setting trigger in {:?} iterations (approx {:?}s)...",
        settings.bb_period, settings.bb_period * settings.time_delta.to_usize().unwrap());

    // Set up connection to FTX API
    let api = if settings.live {
        ftx::rest::Rest::new(ftx::options::Options::from_env())
    } else {
        ftx::rest::Rest::new(
            ftx::options::Options {
                endpoint: ftx::options::Endpoint::Com,
                ..Default::default()
            }
        )
    };

    // Set up bollinger bands
    let mut bb = ta::indicators::BollingerBands::new(
        settings.bb_period,
        settings.bb_std_dev
    ).unwrap();

    let mut count: usize = 0;

    loop {
        count += 1;

        // Get orderbook and handle error
        let order_book = api.request(
            ftx::rest::GetOrderBook {
                market_name: String::from(&settings.market_name),
                depth: Option::from(settings.orderbook_depth),
            }
        ).await;
        let order_book = match order_book {
            Err(e) => {
                // Continue loop is getting orderbook fails
                log::error!("Error: {:?}", e);
                continue;
            }
            Ok(o) => o
        };

        // Calculate values used for analysis
        let perp_delta = (order_book.bids[0].1 - order_book.asks[0].1).to_f64().unwrap();
        let out = ta::Next::next(&mut bb, perp_delta);
        let bb_lower = out.lower;
        let bb_upper = out.upper;

        log::debug!("perp_delta={:.2}, bb_lower={:.2}, bb_upper={:.2}",
            perp_delta, bb_lower, bb_upper);

        if count > settings.bb_period {
            if count == settings.bb_period + 1 {
                log::warn!("Trigger is now set...")
            }

            if perp_delta > bb_upper || perp_delta < bb_lower {
                // Get price and handle error
                let price = api.request(
                    ftx::rest::GetFuture {
                        future_name: String::from(&settings.market_name)
                    }
                ).await;
                let btc_price = match price {
                    Err(e) => {
                        log::error!("Error: {:?}", e);
                        continue;
                    }
                    Ok(o) => o
                };

                let mut price: f64 = 0.0;
                let mut position: helpers::Position = helpers::Position::None;

                if perp_delta > bb_upper {
                    // Enter long position
                    price = btc_price.ask.unwrap().to_f64().unwrap();
                    position = helpers::Position::Long;
                    log::warn!("Perp delta above upper bb, going {:?} at {:.2}", position, price);
                } else if perp_delta < bb_lower {
                    // Enter short position
                    price = btc_price.bid.unwrap().to_f64().unwrap();
                    position = helpers::Position::Short;
                    log::warn!("Perp delta below lower bb, going {:?} at {:.2}", position, price);
                }

                // Write the positions to a csv
                helpers::write_to_csv(
                    &settings.positions_filename,
                    &price,
                    &position,
                ).expect("Unable to write positions to file.");
            }
        }
        std::thread::sleep(std::time::Duration::from_secs(settings.time_delta));
    }
}
