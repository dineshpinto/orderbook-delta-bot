mod helpers;
mod order_handler;

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
    log::info!("{:?}", settings);
    log::info!("Setting trigger in {:?} iterations (approx {:?}s)...",
        settings.bb_period,
        settings.bb_period as u64 * settings.time_delta
    );

    // Set up connection to FTX API
    let api = if settings.live {
        log::info!("The bot is running live");
        dotenv::dotenv().ok();
        ftx::rest::Rest::new(ftx::options::Options::from_env())
    } else {
        log::info!("The bot is not running live, no orders will be placed");
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

    let price_result = api.request(
        ftx::rest::GetFuture {
            future_name: String::from(&settings.market_name)
        }
    ).await.unwrap();

    let price_precision = helpers::convert_increment_to_precision(price_result.price_increment);
    let size_precision = helpers::convert_increment_to_precision(price_result.size_increment);

    // Use MidpointNearestEven rounding (Banker's rounding)
    let order_size = settings.order_size.round_dp(size_precision);

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
                log::info!("Trigger is now set...")
            }

            if perp_delta > bb_upper || perp_delta < bb_lower {
                // Get price and handle error
                let price_result = api.request(
                    ftx::rest::GetFuture {
                        future_name: String::from(&settings.market_name)
                    }
                ).await;
                let (bid_price, ask_price) = match price_result {
                    Err(e) => {
                        log::error!("Error getting price: {:?}", e);
                        continue;
                    }
                    Ok(o) => {
                        (o.bid.unwrap(), o.ask.unwrap())
                    },
                };

                let mut side: helpers::Side = helpers::Side::Buy;
                let mut price = rust_decimal::Decimal::default();

                if perp_delta > bb_upper {
                    // Enter long position
                    side = helpers::Side::Buy;
                    price = ask_price;
                    // Continue if we are already on the same side
                    if side == current_side { continue; } else { current_side = side }

                    log::info!(
                        "Perp delta above upper bb, {:?} at {:?}",
                        side, price
                    );
                } else if perp_delta < bb_lower {
                    // Enter short position
                    side = helpers::Side::Sell;
                    price = bid_price;
                    // Continue if we are already on the same side
                    if side == current_side { continue; } else { current_side = side }

                    log::info!(
                        "Perp delta below lower bb, {:?} at {:?}",
                        side, price
                    );
                }

                // Map our Side enum to FTX's Side enum
                let order_side: ftx::rest::Side = if current_side == helpers::Side::Buy {
                    ftx::rest::Side::Buy
                } else if current_side == helpers::Side::Sell {
                    ftx::rest::Side::Sell
                } else {
                    continue;
                };

                // Calculate static TP and SL for order
                // TODO: Use dynamic TP and SL based on market movements
                let (tp_price, sl_price) = order_handler::calculate_tp_and_sl(
                    price, order_side, settings.tp_percent, settings.sl_percent, price_precision);
                log::info!(
                    "{:?} {:?} {:?} at {:?}. Take profit at {:?} ({:?}%) and stop loss at {:?} ({:?}%)",
                    current_side, order_size, settings.market_name, price, tp_price,
                    settings.tp_percent, sl_price, settings.sl_percent
                );

                if settings.live {
                    // TODO: Use Kelly criterion for order sizing
                    // Place order on FTX
                    let order_placed = futures::executor::block_on(
                        order_handler::place_market_order(
                            &api,
                            &settings.market_name,
                            order_side,
                            order_size,
                        )
                    );
                    match order_placed {
                        Err(e) => {
                            log::error!("Unable to place order, Err: {:?}", e);
                            continue;
                        }
                        Ok(o) => {
                            log::info!("Order placed successfully: {:?}", o);
                        }
                    };

                    // Place trigger orders on FTX
                    let triggers_placed = futures::executor::block_on(
                        order_handler::place_trigger_orders(
                            &api,
                            &settings.market_name,
                            order_side,
                            order_size,
                            tp_price,
                            sl_price,
                        ));

                    // If unable to place TP or SL, cancel all orders
                    // TODO: Market close position in event of failure
                    if !triggers_placed {
                        log::warn!("Cancelling all orders...");
                        let cancel_orders = futures::executor::block_on(
                            order_handler::cancel_all_orders(&api, &settings.market_name));
                        match cancel_orders {
                            Ok(_o) => continue,
                            Err(e) => {
                                log::error!("Unable to cancel orders Err: {:?}, panic", e);
                                panic!()
                            }
                        }
                    }
                }

                // Write the positions to a csv
                helpers::write_to_csv(
                    &settings.positions_filename,
                    rust_decimal::prelude::ToPrimitive::to_f64(&price).unwrap(),
                    &side,
                ).expect("Unable to write positions to file.");
            }
        }
    }
}
