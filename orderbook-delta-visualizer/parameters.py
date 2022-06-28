import datetime
import os
from dataclasses import dataclass
from typing import Union

from strategy import BaseStrategy, BollingerBandStrategy


@dataclass(frozen=True)
class Parameters:
    """ Parameters to use when running visualizer """
    # Name of spot market to track on FTX e.g. BTC/USD, ETH/USD
    spot_market: str = "BTC/USD"
    # Name of futures market to track on FTX e.g. BTC-PERP, ETH-PERP
    perp_future: str = "BTC-PERP"
    # Class of strategy to use
    strategy: BaseStrategy = BollingerBandStrategy(bband_length=20, bband_std=3)
    # Maximum number of data points visible on the screens
    max_visible_length: int = 1000
    # Template for graph theme e.g. plotly_dark, plotly, seaborn
    template: str = "plotly_dark"
    # Size of window in pixels
    window_size: (int, int) = (1400, 850)
    # Log live data to a csv file, use False to disable
    logfile: Union[str, bool] = os.path.join(
        "data",
        f"{datetime.datetime.utcnow().strftime('%Y-%m-%d_%H-%M-%S')}_orderbook_delta_logger_"
        f"{'_'.join(spot_market.split('/'))}_{'_'.join(perp_future.split('-'))}.csv"
    )
