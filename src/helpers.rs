#[derive(serde::Serialize, serde::Deserialize)]
pub(crate) struct SettingsFile {
    /// Format to follow for settings file
    pub(crate) market_name: String,
    pub(crate) time_delta: u64,
    pub(crate) bb_period: usize,
    pub(crate) bb_std_dev: f64,
    pub(crate) orderbook_depth: u32,
    pub(crate) live: bool,
    pub(crate) positions_filename: String,
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum Position {
    /// enum to store current position in market
    Long,
    Short,
    None,
}

impl std::fmt::Display for Position {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Position::Long => write!(f, "long"),
            Position::Short => write!(f, "short"),
            Position::None => write!(f, "none"),
        }
    }
}

pub(crate) fn write_to_csv(filename: &str, price: &f64, position: &Position)
                           -> Result<(), Box<dyn std::error::Error>> {
    /// Write utc time, price and position to a csv file
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