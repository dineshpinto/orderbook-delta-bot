mod helpers;

/// Sets up async function call to FTX
/// Waits for bb_period time steps, then sets trigger
/// Calculates delta at each timestep
#[tokio::main]
async fn main() {
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

    log::info!("Setting trigger in {:?} iterations (approx {:?}s)...",
        settings.bb_period,
        settings.bb_period * rust_decimal::prelude::ToPrimitive::to_usize(
            &settings.time_delta).unwrap()
    );

    // Set up connection to FTX API
    let api = if settings.live {
        log::warn!("The bot is running live");
        dotenv::dotenv().ok();
        ftx::rest::Rest::new(ftx::options::Options::from_env())
    } else {
        log::warn!("The bot is not running live, no orders will be placed");
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
        settings.bb_std_dev,
    ).unwrap();

    let mut count: usize = 0;
    let mut current_side: helpers::Side = helpers::Side::default();

    loop {
        count += 1;
        // Sleep before loop logic to handle continue
        std::thread::sleep(std::time::Duration::from_secs(settings.time_delta));

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
        let perp_delta = rust_decimal::prelude::ToPrimitive::to_f64(
            &(order_book.bids[0].1 - order_book.asks[0].1)).unwrap();
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
                        log::error!("Error getting price: {:?}", e);
                        continue;
                    }
                    Ok(o) => o
                };

                let mut price: f64 = 0.0;
                let mut side: helpers::Side = helpers::Side::Buy;

                if perp_delta > bb_upper {
                    // Enter long position
                    price = rust_decimal::prelude::ToPrimitive::to_f64(&btc_price.ask.unwrap()).unwrap();
                    side = helpers::Side::Buy;
                    // Continue if we are already on the same side
                    if side == current_side { continue; } else { current_side = side }

                    log::warn!(
                        "Perp delta above upper bb, going {:?} at {:.2}",
                        side.to_string(), price
                    );
                } else if perp_delta < bb_lower {
                    // Enter short position
                    price = rust_decimal::prelude::ToPrimitive::to_f64(&btc_price.bid.unwrap()).unwrap();
                    side = helpers::Side::Sell;
                    // Continue if we are already on the same side
                    if side == current_side { continue; } else { current_side = side }

                    log::warn!(
                        "Perp delta below lower bb, going {:?} at {:.2}",
                        side.to_string(), price
                    );
                }

                if settings.live {
                    let _side = if side == helpers::Side::Buy {
                        ftx::rest::Side::Buy
                    } else if side == helpers::Side::Sell {
                        ftx::rest::Side::Sell
                    } else {
                        continue;
                    };

                    let order_placed = api.request(ftx::rest::PlaceOrder {
                        market: String::from(&settings.market_name),
                        side: _side,
                        price: None,
                        r#type: Default::default(),
                        size: Default::default(),
                        reduce_only: true,
                        ioc: false,
                        post_only: false,
                        client_id: None,
                        reject_on_price_band: false,
                    }).await;

                    match order_placed {
                        Err(e) => {
                            log::error!("Unable to place order, Err: {:?}", e);
                            continue;
                        }
                        Ok(o) => {
                            log::warn!("Order placed successfully: {:?}", o);
                        }
                    }
                }

                // Write the positions to a csv
                helpers::write_to_csv(
                    &settings.positions_filename,
                    &price,
                    &side,
                ).expect("Unable to write positions to file.");
            }
        }
    }
}
