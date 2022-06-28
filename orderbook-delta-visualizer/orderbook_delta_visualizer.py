import csv
import dash
import datetime
import plotly.express as px
import plotly.graph_objects as go
import plotly.io as pio
from collections import deque
from dash import dcc, html
from dash.dependencies import Output, Input
from ftx import FtxClient
from plotly.subplots import make_subplots
from typing import Tuple

from ftx_websocket_client import FtxWebsocketClient
from orderbook_delta_strategies import Position, Parameters, BaseStrategy


def get_bid_ask_and_delta(market: str) -> Tuple[float, float, float, float, float]:
    orderbook = ftx.get_orderbook(market)

    bid_price, bid_volume = orderbook["bids"][0]
    ask_price, ask_volume = orderbook["asks"][0]

    delta = bid_volume - ask_volume
    return bid_price, ask_price, bid_volume, ask_volume, delta


def update_deque_lists():
    """ Query websocket and update deque lists """
    try:
        spot_bid_price, spot_ask_price, spot_bid_volume, spot_ask_volume, spot_delta = get_bid_ask_and_delta(
            SPOT_MARKET)
        perp_bid_price, perp_ask_price, perp_bid_volume, perp_ask_volume, perp_delta = get_bid_ask_and_delta(
            PERP_FUTURE)
        utc_timestamp = datetime.datetime.utcnow()
    except IndexError:
        # Catch errors from websocket and handle them by skipping over the point
        pass
    else:
        # If no error, append data into deque lists
        utc_timestamps.append(utc_timestamp)

        spot_bids.append(spot_bid_price)
        spot_asks.append(spot_ask_price)
        spot_ask_volumes.append(spot_ask_volume)
        spot_bid_volumes.append(spot_bid_volume)
        spot_deltas.append(spot_delta)

        perp_bids.append(perp_bid_price)
        perp_asks.append(perp_ask_price)
        perp_ask_volumes.append(perp_ask_volume)
        perp_bid_volumes.append(perp_bid_volume)
        perp_deltas.append(perp_delta)

        if LOGFILE:
            with open(LOGFILE, "a") as _file:
                _writer = csv.writer(_file, delimiter=',')
                _writer.writerow([utc_timestamp, spot_bid_price, spot_ask_price, spot_bid_volume, spot_ask_volume,
                                  perp_bid_price, perp_ask_price, perp_bid_volume, perp_ask_volume])


app = dash.Dash(__name__)

app.layout = html.Div(
    [
        dcc.Graph(id='live-graph'),
        dcc.Interval(id='graph-update', interval=1000, n_intervals=0),
    ]
)


@app.callback(
    Output('live-graph', 'figure'),
    [Input('graph-update', 'n_intervals')]
)
def update_graph_scatter(_):
    update_deque_lists()
    position.append(STRATEGY.strategy(perp_deltas=list(perp_deltas), spot_deltas=list(spot_deltas)))

    fig = make_subplots(rows=5, cols=1, shared_xaxes=True,
                        subplot_titles=(
                            f"{SPOT_MARKET} price", f"{SPOT_MARKET} volume", f"{PERP_FUTURE} price",
                            f"{PERP_FUTURE} volume", f" Delta volume (+ {STRATEGY})"))

    fig = STRATEGY.plot_strategy(timestamps=list(utc_timestamps), fig=fig)

    # Spot bids and asks
    fig.add_trace(
        go.Scatter(
            x=list(utc_timestamps),
            y=list(spot_bids),
            name='Spot Bids',
            mode='lines',
            line=dict(color=px.colors.qualitative.Plotly[2])
        ),
        row=1,
        col=1
    )
    fig.add_trace(
        go.Scatter(
            x=list(utc_timestamps),
            y=list(spot_asks),
            name='Spot Asks',
            mode='lines',
            line=dict(color=px.colors.qualitative.Plotly[4])
        ),
        row=1,
        col=1
    )

    # Spot Volumes
    fig.add_trace(
        go.Scatter(
            x=list(utc_timestamps),
            y=list(spot_ask_volumes),
            name='Spot Ask Volume',
            mode='lines',
            line=dict(color=px.colors.qualitative.Plotly[6])
        ),
        row=2,
        col=1
    )
    fig.add_trace(
        go.Scatter(
            x=list(utc_timestamps),
            y=list(spot_bid_volumes),
            name='Spot Bid Volume',
            mode='lines',
            line=dict(color=px.colors.qualitative.Plotly[2])
        ),
        row=2,
        col=1
    )

    # Perp bids and asks
    fig.add_trace(
        go.Scatter(
            x=list(utc_timestamps),
            y=list(perp_bids),
            name='Perp Bids',
            mode='lines',
            line=dict(color=px.colors.qualitative.Plotly[2])
        ),
        row=3,
        col=1
    )
    fig.add_trace(
        go.Scatter(
            x=list(utc_timestamps),
            y=list(perp_asks),
            name='Perp Asks',
            mode='lines',
            line=dict(color=px.colors.qualitative.Plotly[4])
        ),
        row=3,
        col=1
    )

    # Volumes
    fig.add_trace(
        go.Scatter(
            x=list(utc_timestamps),
            y=list(perp_ask_volumes),
            name='Perp Ask Volume',
            mode='lines',
            line=dict(color=px.colors.qualitative.Plotly[6])
        ),
        row=4,
        col=1
    )
    fig.add_trace(
        go.Scatter(
            x=list(utc_timestamps),
            y=list(perp_bid_volumes),
            name='Perp Bid Volume',
            mode='lines',
            line=dict(color=px.colors.qualitative.Plotly[2])
        ),
        row=4,
        col=1
    )

    __current_pos = None
    for idx, pos in enumerate(position):
        if pos == Position.LONG and __current_pos != Position.LONG:
            __current_pos = Position.LONG
            fig.add_vline(utc_timestamps[idx + 1], line_color=px.colors.qualitative.Plotly[2],
                          line_dash="dot", row="all", col=1)
        elif pos == Position.SHORT and __current_pos != Position.SHORT:
            __current_pos = Position.SHORT
            fig.add_vline(utc_timestamps[idx + 1], line_color=px.colors.qualitative.Plotly[6],
                          line_dash="dot", row="all", col=1)

    fig['layout'].update(
        title_text=f"{SPOT_MARKET} and {PERP_FUTURE} orderbook at depth=1",
        xaxis=dict(range=[min(utc_timestamps), max(utc_timestamps)]),
        width=WINDOW_SIZE[0],
        height=WINDOW_SIZE[1],
        transition_duration=500,
    )

    return fig


if __name__ == '__main__':
    ftx_rest = FtxClient()
    ftx_markets = [market["name"] for market in ftx_rest.get_markets()]
    del ftx_rest

    # Basic input sanity check
    assert Parameters.template in pio.templates, f"Invalid plotly template {Parameters.template}, Valid {pio.templates}"
    assert Parameters.spot_market in ftx_markets, f"Invalid spot market {Parameters.spot_market}, Valid {ftx_markets}"
    assert Parameters.perp_future in ftx_markets, f"Invalid perp future {Parameters.perp_future}, Valid {ftx_markets}"
    assert 1 < Parameters.max_visible_length < 5000, \
        f"Invalid visible length {Parameters.max_visible_length}, visible length should be between 1 and 5000"
    assert Parameters.window_size[0] > 0 and Parameters.window_size[1] > 0, \
        f"Invalid window size {Parameters.window_size}"
    assert isinstance(Parameters.strategy, BaseStrategy), \
        f"Strategy {Parameters.strategy} must be a subclass of BaseStrategy"
    assert isinstance(Parameters.logfile, bool) or isinstance(Parameters.logfile, str), \
        f"Invalid logfile {Parameters.logfile}"

    # Set up global params from Parameters dataclass
    pio.templates.default = Parameters.template
    SPOT_MARKET = Parameters.spot_market
    PERP_FUTURE = Parameters.perp_future
    STRATEGY = Parameters.strategy
    MAX_VISIBLE_LENGTH = Parameters.max_visible_length
    WINDOW_SIZE = Parameters.window_size
    LOGFILE = Parameters.logfile

    if LOGFILE:
        with open(LOGFILE, "w") as file:
            writer = csv.writer(file, delimiter=',')
            writer.writerow(["utc_timestamp", "spot_bid_price", "spot_ask_price", "spot_bid_volume", "spot_ask_volume",
                             "perp_bid_price", "perp_ask_price", "perp_bid_volume", "perp_ask_volume"])

    # Initialize FTX websocket and deque lists
    ftx = FtxWebsocketClient()

    utc_timestamps = deque(maxlen=MAX_VISIBLE_LENGTH)
    spot_bids = deque(maxlen=MAX_VISIBLE_LENGTH)
    spot_asks = deque(maxlen=MAX_VISIBLE_LENGTH)
    perp_bids = deque(maxlen=MAX_VISIBLE_LENGTH)
    perp_asks = deque(maxlen=MAX_VISIBLE_LENGTH)
    spot_deltas = deque(maxlen=MAX_VISIBLE_LENGTH)
    perp_deltas = deque(maxlen=MAX_VISIBLE_LENGTH)
    perp_ask_volumes = deque(maxlen=MAX_VISIBLE_LENGTH)
    perp_bid_volumes = deque(maxlen=MAX_VISIBLE_LENGTH)
    spot_bid_volumes = deque(maxlen=MAX_VISIBLE_LENGTH)
    spot_ask_volumes = deque(maxlen=MAX_VISIBLE_LENGTH)
    position = deque(maxlen=MAX_VISIBLE_LENGTH)

    update_deque_lists()

    app.run_server(use_reloader=True)
