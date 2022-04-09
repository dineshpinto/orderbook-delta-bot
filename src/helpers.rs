use rust_decimal::Decimal;

/// Format to follow for settings file
#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub(crate) struct SettingsFile {
    pub(crate) market_name: String,
    pub(crate) time_delta: u64,
    pub(crate) bb_period: usize,
    pub(crate) bb_std_dev: f64,
    pub(crate) orderbook_depth: u32,
    pub(crate) live: bool,
    pub(crate) order_size: Decimal,
    pub(crate) tp_percent: f64,
    pub(crate) sl_percent: f64,
    pub(crate) positions_filename: String,
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
            Side::Buy => write!(f, "long"),
            Side::Sell => write!(f, "short"),
            Side::None => write!(f, "none"),
        }
    }
}


impl Default for Side {
    fn default() -> Side {
        Side::None
    }
}

/// Write utc time, price and position to a csv file
pub(crate) fn write_to_csv(filename: &str, price: &f64, position: &Side)
                           -> Result<(), Box<dyn std::error::Error>> {
    let utc_time: chrono::prelude::DateTime<chrono::prelude::Utc> = chrono::prelude::Utc::now();

    let file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .append(true)
        .open(String::from(filename))
        .unwrap();
    let mut wtr = csv::Writer::from_writer(file);
    log::debug!("Writing position to {:?}", String::from(filename));
    wtr.write_record(&[utc_time.to_string(), price.to_string(), position.to_string()])?;
    wtr.flush()?;
    Ok(())
}