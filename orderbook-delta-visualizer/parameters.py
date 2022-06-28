import datetime
import os
from dataclasses import dataclass
from typing import Union

from strategy import BaseStrategy, BollingerBandStrategy


def get_formatted_filepath(folder: str, base_filename: str, spot_market: str, perp_future: str) -> str:
    """ Example output filename: 2022-06-28_08-52-21_BTC-USD_BTC-PERP_orderbook_delta_logger.csv """
    filename = f"{datetime.datetime.utcnow().strftime('%Y-%m-%d_%H-%M-%S')}_{'-'.join(spot_market.split('/'))}_" \
               f"{'-'.join(perp_future.split('-'))}_{base_filename}.csv"
    return os.path.join(folder, filename)


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
    logfile: Union[str, bool] = get_formatted_filepath(
        folder="data",
        base_filename="orderbook_delta_logger",
        spot_market=spot_market,
        perp_future=perp_future
    )
