# Orderbook Delta Visualizer

A GUI visualizer written in Python using Dash and Plotly.

## Installation

### Install poetry (if you haven't already)
Follow the installation instructions on the [poetry website](https://python-poetry.org/docs/).

### Install all dependencies
```shell
poetry install
```
### Run the visualizer
```shell
poetry run python orderbook_delta_visualizer.py
```

This will start a dash server, which you can open in your browser.

## Usage

### To modify strategy
- The abstract base class `strategy.py/BaseStrategy` defines all strategies
- Create a new class inheriting from `BaseStrategy` abstract base class
- Create all required functions as defined in the base class
- Update the `strategy` attribute of `parameters.py/Parameters` dataclass to point to the new strategy

### To modify parameters
- All parameters are stored in `parameters.py/Parameters`
- All parameters can be updated live, the server will restart automatically