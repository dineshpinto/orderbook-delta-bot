# Orderbook Delta Bot

A trading bot written in Rust. 

The strategy based on the concept of *mean reversion*. We look for large deviations in the volume delta of BTC-PERP on 
FTX at a depth of 1. 
These deviations could be caused by over-enthusiastic and over-leveraged market participants.

We counter-trade those deviations, and enter short/long positions based on triggers given by a large deviation (> 2 SDs) 
on the orderbook delta 
from a (10-20) period rolling bollinger band.

We are testing this with BTC-PERP on FTX, which has good liquidity and small spreads. 
In principle, the scheme could be modified for lower liquidity pairs too, perhaps by adjusting the bollinger band width 
and length for generating triggers.

We use the definitions : 

| Name         | Definition                                                     |
|--------------|----------------------------------------------------------------|
| `delta_perp` | Difference between bid and ask volume at depth = 1 on BTC-PERP |
| `bb_upper`   | Upper bollinger band of `delta_perp`                           |
| `bb_lower`   | Lower bollinger band of `delta_perp`                           |

| Trigger                   | Position |
|---------------------------|----------|
| `delta_perp` > `bb_upper` | short    |
| `delta_perp` < `bb_lower` | long     |

A full analysis of this strategy is detailed in [here](https://github.com/dineshpinto/market-analytics).

## Installation
1. Install all dependencies 
```rust
cargo build
```
2. Run script
```rust
cargo run
```

## Options
`settings.json` contains all the configurable options:

| Option               | Explanation                                                            |
|----------------------|------------------------------------------------------------------------|
| `market_name`        | Name of futures market on FTX (default: BTC-PERP)                      |
| `time_delta`         | Delay in seconds between queries (default: 5)                          |
| `bb_period`          | Bollinger band period (default: 20)                                    |
| `bb_std_dev`         | Bollinger band standard deviation (default: 2)                         |
| `orderbook_depth`    | Depth of orderbook to query (default: 1)                               |
| `live`               | Place live orders on FTX, requires API keys in `.env` (default: false) |
| `positions_filename` | Store positions in a csv file for further analysis                     |

## Disclaimer
This project is only for educational purposes. There is no guarantee of the accuracy of the output data. Do not make 
any trading or investment decisions based on these results. Always do your own research.
