//! A set of functions to handle config files, saving data and additional math

/// Format to follow for settings JSON file
#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub(crate) struct SettingsFile {
    /// Name of futures market on FTX
    pub(crate) market_name: String,
    /// Time (in seconds) to sample orderbook, each sample is 1s
    pub(crate) sampling_time: u64,
    /// Period of bollinger band
    pub(crate) bb_period: usize,
    /// Standard deviation of bollinger band
    pub(crate) bb_std_dev: f64,
    /// Depth of orderbook to sum
    pub(crate) orderbook_depth: u32,
    /// Make live trades or not
    pub(crate) live: bool,
    /// Size of position to take
    pub(crate) order_size: rust_decimal::Decimal,
    /// Percent to take profit at
    pub(crate) tp_percent: rust_decimal::Decimal,
    /// Percent to stop loss at
    pub(crate) sl_percent: rust_decimal::Decimal,
    /// Store positions in csv (positions.csv by default)
    pub(crate) write_to_file: bool,
}


/// enum to store current position in market
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Side {
    Buy,
    Sell,
    None,
}

impl std::fmt::Display for Side {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Side::Buy => write!(f, "buy"),
            Side::Sell => write!(f, "sell"),
            Side::None => write!(f, "none"),
        }
    }
}

impl Default for Side {
    fn default() -> Side {
        Side::None
    }
}


/// Write utc time, price, size and current position to a csv file
pub(crate) fn write_to_csv(
    filename: &str,
    price: rust_decimal::Decimal,
    size: rust_decimal::Decimal,
    side: &Side,
    positions_count: usize) -> Result<(), Box<dyn std::error::Error>> {
    let utc_time: chrono::prelude::DateTime<chrono::prelude::Utc> = chrono::prelude::Utc::now();

    // Delete any existing file on first run
    if positions_count == 1 as usize {
        let remove_file = std::fs::remove_file(filename);
        match remove_file {
            Err(_e) => {
                log::info!("Positions file does not exist, creating new file")
            }
            Ok(o) => o
        }
    }

    // Append to existing file, or create new file
    let file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .append(true)
        .open(String::from(filename))
        .unwrap();

    log::debug!("Writing position to {:?}", String::from(filename));

    let mut wtr = csv::Writer::from_writer(file);

    // On first run, write header
    if positions_count == 1 as usize {
        wtr.write_record(&["utc_time", "price", "size", "side"])?;
    }
    // Write row
    wtr.write_record(
        &[
            utc_time.to_string(),
            price.to_string(),
            size.to_string(),
            side.to_string()
        ]
    )?;
    wtr.flush()?;
    Ok(())
}


/// Convert an increment to a precision
///
/// eg.
///     increment=0.0001 has precision=4 and
///     increment=1 has precision=0
pub(crate) fn convert_increment_to_precision(increment: rust_decimal::Decimal) -> u32 {
    let mut precision = 0;
    let mut incr = increment;

    while incr < rust_decimal::Decimal::from(1) {
        incr *= rust_decimal::Decimal::from(10);
        precision += 1;
    }
    return precision;
}

/// Lead setting file from JSON
pub(crate) fn read_settings(filepath: &str) -> SettingsFile {
    let settings_filepath = std::path::Path::new(&filepath);
    let settings_file = std::fs::File::open(settings_filepath)
        .expect("Config file not found");
    let reader = std::io::BufReader::new(settings_file);
    let settings: SettingsFile =
        serde_json::from_reader(reader).expect("Error when reading config json");

    return settings;
}

/// Invert side i.e. buy -> sell, sell -> buy
pub(crate) fn invert_side(side: ftx::rest::Side) -> ftx::rest::Side {
    return match side {
        ftx::rest::Side::Buy => ftx::rest::Side::Sell,
        ftx::rest::Side::Sell => ftx::rest::Side::Buy
    };
}