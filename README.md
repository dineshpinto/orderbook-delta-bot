# Orderbook Delta Bot

A trading bot written in Rust. 

The strategy based on the concept of *mean reversion*. We look for large deviations in the volume delta of BTC-PERP on FTX at a depth of 1. 
These deviations could be caused by over-enthusiastic and over-leveraged market participants.

We counter-trade those deviations, and enter short/long positions based on triggers given by a large delta (> 2 SDs) 
from a (10-20) period rolling bollinger band.

We are testing this with BTC-PERP on FTX, which has good liquidity and small spreads. 
In principle, the scheme could be modified for lower liquidity pairs too, perhaps by adjusting the bollinger band width and length for generating triggers.

We use the definitions : 

| Name | Definition |
| --- | --- |
`delta_perp`| Difference between bid and ask volume at depth = 1 on BTC-PERP
`bb_upper` | Upper bollinger band of `delta_perp`
`bb_lower` | Lower bollinger band of `delta_perp`


| Trigger | Position |
| --- | --- |
`delta_perp` > `bb_upper` | short
`delta_perp` < `bb_lower` | long

A full analysis of this strategy is detailed in this [Jupyter Notebook](https://github.com/dineshpinto/market-analytics/blob/main/notebooks/OrderbookDeltaAnalyzer.ipynb).