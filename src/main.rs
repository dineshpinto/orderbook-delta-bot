//! A trading bot written in Rust.
//!
//! © Dinesh Pinto, 2022
//! LICENSE: Apache-2.0, see http://www.apache.org/licenses/LICENSE-2.0
//!
//! Uses a trading strategy based on counter trading large deviations in
//! the delta volume (i.e. bid volume - ask volume) on the futures orderbook.
//!
//! The bot waits for deviations outside a specified standard deviation of a bollinger band
//! and enters a long/short position accordingly.
//!
//! A full analysis of this strategy is given in
//! [dineshpinto/market-analytics](https://github.com/dineshpinto/market-analytics)

mod helpers;
mod order_handler;
mod tests;

/// Core logical loop for the bot.
///
/// The process is:
/// 1. Set up asynchronous connection to the FTX API
/// 2. Wait for N time steps, where N is the length of the bollinger band.
/// 3. Set the trigger to ready the bot
/// 4. Enter a long/short position based on the entry conditions specified in settings
/// 5. Exit by either hitting the take profit/stop loss or by switching sides
///
/// Note: For risk management purposes, in case the bot is unable to place TP/SL after
/// opening a position it will close the open position and panic.
///
#[tokio::main]
async fn main() {
    // Load settings file
    let settings_filepath = String::from("settings.json");
    let settings = helpers::read_settings(&settings_filepath);

    // Set up logging
    let mut builder = env_logger::Builder::new();
    builder
        .filter(None, log::LevelFilter::Info)
        .write_style(env_logger::WriteStyle::Always)
        .target(env_logger::Target::Stdout)
        .init();

    log::info!("Settings file loaded from {:?}.", settings_filepath);
    log::info!("{:?}", settings);

    // Set up connection to FTX API
    let api = if settings.live {
        // Read .env file for API keys if bot is live
        log::info!("The bot is running live");
        dotenv::dotenv().ok();
        ftx::rest::Rest::new(ftx::options::Options::from_env())
    } else {
        // Use public endpoint if bot is not live
        log::info!("The bot is not running live, no orders will be placed");
        ftx::rest::Rest::new(
            ftx::options::Options {
                endpoint: ftx::options::Endpoint::Com,
                ..Default::default()
            }
        )
    };

    // Get precision for price and size for current market,
    // use MidpointNearestEven rounding (Banker's rounding)
    let future_result = api.request(
        ftx::rest::GetFuture {
            future_name: String::from(&settings.market_name)
        }
    ).await.unwrap();

    // Set precision for price
    let price_precision = helpers::convert_increment_to_precision(
        future_result.price_increment);

    // Set precision for order

    // Panic if order size is too small
    if settings.order_size < future_result.size_increment {
        log::error!(
            "Order size is smaller than minimum order size ({:?} < {:?})",
            settings.order_size, future_result.size_increment
        );
        panic!();
    }
    // Orders with size_increment < 1 need to handled separately from size_increment > 1
    let mut _order_size = rust_decimal::Decimal::from(0);
    if future_result.size_increment < rust_decimal::Decimal::from(1) {
        let size_precision = helpers::convert_increment_to_precision(
            future_result.size_increment);
        _order_size = settings.order_size.round_dp(size_precision);
    } else {
        _order_size = (future_result.size_increment * settings.order_size).round()
            / future_result.size_increment;
    }

    // Set up bollinger bands
    let mut bb = ta::indicators::BollingerBands::new(
        settings.bb_period,
        settings.bb_std_dev,
    ).unwrap();

    // Set up loop outer variables
    let mut count: usize = 0;
    let mut positions_count: usize = 0;
    let mut current_side: helpers::Side = helpers::Side::default();
    let mut price = rust_decimal::Decimal::default();

    log::info!("Setting trigger in {:?} iterations (approx {:?}s)...",
        settings.bb_period,
        settings.bb_period as u64 * settings.time_delta
    );

    loop {
        count += 1;
        // Sleep before loop logic to handle continue statements
        std::thread::sleep(std::time::Duration::from_secs(settings.time_delta));

        // Get orderbook
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

        // Only perform further calculation if bb_period is passed
        if count > settings.bb_period {
            if count == settings.bb_period + 1 {
                log::info!("Trigger is now set...")
            }

            // Entry conditions
            if perp_delta > bb_upper || perp_delta < bb_lower {
                // Get current price
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
                    }
                };

                // Create local variables to handle side
                let mut _side: helpers::Side = helpers::Side::Buy;

                if perp_delta > bb_upper {
                    // Enter short position
                    _side = helpers::Side::Sell;
                    price = bid_price;

                    // Continue if we are already on the same side, else change side
                    if _side == current_side { continue; } else { current_side = _side }

                    log::info!(
                        "Perp delta above upper bb, {:?} at {:?}",
                        _side, price
                    );
                } else if perp_delta < bb_lower {
                    // Enter long position
                    _side = helpers::Side::Buy;
                    price = ask_price;

                    // Continue if we are already on the same side, else change side
                    if _side == current_side { continue; } else { current_side = _side }

                    log::info!(
                        "Perp delta below lower bb, {:?} at {:?}",
                        _side, price
                    );
                }

                // Map our Side enum onto FTX Side enum
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
                    "{:?} {:?} {} at {:?}. Take profit at {:?} ({:?}%) and \
                    stop loss at {:?} ({:?}%)",
                    current_side, _order_size, settings.market_name, price, tp_price,
                    settings.tp_percent, sl_price, settings.sl_percent
                );
                positions_count += 1;

                if settings.live {
                    // Check if position is currently open and close it
                    let open_position = futures::executor::block_on(
                        order_handler::get_open_position(&api, &settings.market_name));

                    if open_position {
                        log::info!("Closing existing position...");
                        futures::executor::block_on(
                            order_handler::market_close_order(
                                &api, &settings.market_name,
                            )
                        );
                        futures::executor::block_on(
                            order_handler::cancel_all_trigger_orders(
                                &api, &settings.market_name,
                            )
                        );
                    }

                    // TODO: Use Kelly criterion for order sizing
                    // Place order on FTX
                    let order_placed = futures::executor::block_on(
                        order_handler::place_market_order(
                            &api,
                            &settings.market_name,
                            order_side,
                            _order_size,
                        )
                    );

                    if !order_placed {
                        log::warn!("Unable to place order, will continue with loop...");
                        continue;
                    }

                    // Place trigger orders on FTX
                    let triggers_placed = futures::executor::block_on(
                        order_handler::place_trigger_orders(
                            &api,
                            &settings.market_name,
                            order_side,
                            _order_size,
                            tp_price,
                            sl_price,
                        )
                    );

                    // If unable to place TP or SL, cancel all orders
                    // TODO: Market close position in event of failure
                    if !triggers_placed {
                        log::warn!("Cancelling all orders...");
                        let order_closed = futures::executor::block_on(
                            order_handler::market_close_order(&api, &settings.market_name));
                        let triggers_cancelled = futures::executor::block_on(
                            order_handler::cancel_all_trigger_orders(
                                &api, &settings.market_name,
                            )
                        );

                        if order_closed && triggers_cancelled {
                            continue;
                        } else {
                            log::error!("Unable to close order, panicking!");
                            panic!()
                        }
                    }
                }

                // Write the positions to a csv
                if settings.write_to_file {
                    helpers::write_to_csv(
                        "positions.csv",
                        price,
                        _order_size,
                        &current_side,
                        positions_count,
                    ).expect("Unable to write positions to file.");
                }
            }
        }
    }
}
