# Orderbook Delta Bot

A trading bot written in Rust 🦀.

The strategy based on the concept of *mean reversion*. We look for large deviations in the volume delta of BTC-PERP on
FTX until a defined depth.
These deviations could be caused by over-enthusiastic and over-leveraged market participants (speculation).

We counter-trade those deviations, and enter short/long positions based on triggers given by a large deviation
(> 2 SDs) on the orderbook delta from a 20 period rolling bollinger band.

We are testing this with BTC-PERP on FTX, which has good liquidity and small spreads (~~and FTX~~ now Binance, praise be to CZ, has the best API
in the business). In principle, the scheme could be modified for lower liquidity pairs too, perhaps by adjusting
the sampling period and market depth for generating triggers.

We use the definitions:

| Name            | Definition                                                             |
|-----------------|------------------------------------------------------------------------|
| `bid_ask_delta` | Difference between the sum of bid and ask volumes till a defined depth |
| `bb.upper`      | Upper bollinger band (L=20, SD=2) of `bid_ask_delta`                   |
| `bb.lower`      | Lower bollinger band (L=20, SD=2) of `bid_ask_delta`                   |

| Trigger                    | Position |
|----------------------------|----------|
| `bid_ask_delta > bb.upper` | short    |
| `bid_ask_delta < bb.lower` | long     |

A full analysis of this strategy along with its limitations in
[dineshpinto/market-analytics](https://github.com/dineshpinto/market-analytics).

## Installation

### Clone the repository

#### With Git

```shell
git clone https://github.com/dineshpinto/orderbook-delta-bot.git
```

#### With GitHub CLI

```shell
gh repo clone dineshpinto/orderbook-delta-bot
```

### Set up bot

#### Bot settings

Rename `settings-example.json` to `settings.json`. The default settings are given below.

#### Place live orders (optional)

- Rename `.env.example` to `.env`, and enter in your FTX API keys
- Set `"live" : true` in `settings.json`

### Install all dependencies and build

```shell
cargo build
```

### 🫡 Run script

```shell
cargo run
```

## Orderbook Delta GUI (optional delta strategy tester)

To test out new delta strategies and visualize them live, use the orderbook-delta-visualizer. 
It's written in Python, with plotting handled by Dash and Plotly, and contains a set of configurable parameters 
and strategies. See `orderbook-delta-visualizer/` for more details.

[GUI](https://user-images.githubusercontent.com/15251343/176155957-e6096eb1-a1ef-4373-b66e-7ebaa83b5b84.mov)

## Settings

`settings.json` contains all the configurable options:

| Name              | Explanation                                                            |
|-------------------|------------------------------------------------------------------------|
| `market_name`     | Name of futures market on FTX (default: BTC-PERP)                      |
| `sampling_time`   | Time (in seconds) to sample orderbook, each sample is 1s (default: 60) |
| `bb_period`       | Bollinger band period (default: 20)                                    |
| `bb_std_dev`      | Bollinger band standard deviation (default: 2)                         |
| `orderbook_depth` | Depth of orderbook to sum (default: 5)                                 |
| `live`            | Place live orders on FTX, requires API keys in `.env` (default: false) |
| `order_size`      | Size of order to place (default: 0.1618 BTC)                           |
| `tp_percent`      | Percent move to take profit at (default: 0.2%)                         |
| `sl_percent`      | Percent move to stop loss at (default: 0.1%)                           |
| `write_to_file`   | Store positions in a csv file for further analysis (default: true)     |

## TODO

- [ ] Use Kelly criterion for order sizing (probabilities can be estimated from prior analysis)
- [ ] Use dynamic take profit and stop loss based on market movement (this is simply used as protection from getting
  rekt, not as actual exit points)
- [ ] Perform spectral analysis with wider timeframes to identify optimal
  market conditions
- [ ] Switch to websockets API for reduced data query lag
- [ ] For more high frequency applications, switching to a library
  like [ccapi](https://github.com/crypto-chassis/ccapi/) is handy. Unfortunately this only exists for C++ right now.

## Disclaimer

This project is for educational purposes only. You should not construe any such information or other material as legal,
tax, investment, financial, or other advice. Nothing contained here constitutes a solicitation, recommendation,
endorsement, or offer by me or any third party service provider to buy or sell any securities or other financial
instruments in this or in any other jurisdiction in which such solicitation or offer would be unlawful under the
securities laws of such jurisdiction.

If you plan to use real money, use at your own risk.

Under no circumstances will I be held responsible or liable in any way for any claims, damages, losses, expenses, costs,
or liabilities whatsoever, including, without limitation, any direct or indirect damages for loss of profits.
