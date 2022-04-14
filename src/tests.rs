#[cfg(test)]
mod test_helpers {
    use crate::helpers::{convert_increment_to_precision, read_settings, SettingsFile, write_to_csv};

    #[test]
    fn test_convert_increment_to_precision() {
        let value = rust_decimal::prelude::FromPrimitive::from_f64(0.1).unwrap();
        let precision = convert_increment_to_precision(value);
        assert_eq!(precision, 1 as u32);
    }

    #[test]
    fn test_write_to_csv() {
        // Create a test file
        let filename = "test_write_to_csv.csv";
        write_to_csv(
            filename,
            rust_decimal::Decimal::from(10 as i64),
            rust_decimal::Decimal::from(10 as i64),
            &crate::helpers::Side::Sell,
            1 as usize,
        ).unwrap();

        // Verify the file, and delete it
        let mut rdr = csv::Reader::from_path(filename).unwrap();
        for result in rdr.records() {
            let record = result.unwrap();
            // Only compare two records
            assert_eq!(record[1], "10".to_string());
            assert_eq!(record[2], "10".to_string());
        };

        std::fs::remove_file(filename).unwrap();
    }

    #[test]
    fn test_read_settings() {
        // Create a test file
        let filename = "test_read_settings.json";
        let data = SettingsFile {
            market_name: "BTC-USD".to_string(),
            time_delta: 1,
            bb_period: 10,
            bb_std_dev: 0.0,
            orderbook_depth: 0,
            live: false,
            order_size: Default::default(),
            tp_percent: Default::default(),
            sl_percent: Default::default(),
            write_to_file: false,
        };
        serde_json::to_writer_pretty(
            &std::fs::File::create(filename).unwrap(), &data).unwrap();

        // Verify the test file, and delete it
        let settings = read_settings(filename);
        assert_eq!(settings.time_delta, 1 as u64);
        assert_eq!(settings.bb_period, 10 as usize);
        assert_eq!(settings.bb_std_dev, 0 as f64);
        assert_eq!(settings.orderbook_depth, 0 as u32);
        std::fs::remove_file(filename).unwrap();
    }
}