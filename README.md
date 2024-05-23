# e2020-data-viewer

[![](https://github.com/ECCC-RPE-EPR/e2020-data-viewer/workflows/CI/badge.svg)](https://github.com/ECCC-RPE-EPR/e2020-data-viewer/actions?query=workflow%3ACI)
[![](https://img.shields.io/github/license/ECCC-RPE-EPR/e2020-data-viewer)](./LICENSE)
[![](https://img.shields.io/github/v/release/ECCC-RPE-EPR/e2020-data-viewer)](https://github.com/ECCC-RPE-EPR/e2020-data-viewer/releases/latest)
[![](https://img.shields.io/static/v1?label=platform&message=linux-64%20|%20osx-64%20|%20win-64&color=lightgrey)](https://github.com/ECCC-RPE-EPR/e2020-data-viewer/releases/latest)
[![](https://img.shields.io/github/languages/top/ECCC-RPE-EPR/e2020-data-viewer)](https://github.com/ECCC-RPE-EPR/e2020-data-viewer)
[![](https://img.shields.io/github/downloads/ECCC-RPE-EPR/e2020-data-viewer/total)](https://github.com/ECCC-RPE-EPR/e2020-data-viewer/releases/latest)

A TUI data viewer for ENERGY2020.

https://github.com/ECCC-RPE-EPR/e2020-data-viewer/assets/1813121/7cd1fe33-f858-4bf1-96ea-403d093e0197

https://github.com/ECCC-RPE-EPR/e2020-data-viewer/assets/1813121/3083e1b5-9fff-4187-8a83-513f6fd1433b

## Install

- Download the precompiled binary from the latest release: https://github.com/ECCC-RPE-EPR/e2020-data-viewer/releases/latest
- Extract the binary from a corresponding file based on your operating system.
- Place the binary in a folder of your choice.
  - (optional) Add the folder to your PATH

## Usage

From any command prompt or terminal, you can run `e2020-data-viewer --help`:

```
$ e2020-data-viewer --help

A TUI for viewing data from ENERGY2020

Usage: e2020-data-viewer [OPTIONS] --file <FILE>

Options:
  -f, --file <FILE>              The input file to use
      --tick-rate <TICK_RATE>    Tick rate (ticks per second) [default: 4]
      --frame-rate <FRAME_RATE>  Frame rate (frames per second) [default: 4]
  -d, --dataset <DATASET>        The dataset to read on load (optional)
  -h, --help                     Print help
  -V, --version                  Print version
```

To load a file:

```
$ e2020-data-viewer --file ./path/to/database.hdf5
```

To load a specific dataset in a file:

```
$ e2020-data-viewer --file ./path/to/database.hdf5 --dataset "routput/Dmd"
```

## Background

`e2020-data-viewer` is a terminal user interface to interactively explore the data produced by a Julia port of [ENERGY2020](https://www.energy2020.com/energy-2020).
ENERGY2020 is used for [Canada's greenhouse gas emissions projections](https://www.canada.ca/en/environment-climate-change/services/climate-change/greenhouse-gas-emissions/projections.html) as part of the [E3MC model](https://www.canada.ca/en/services/environment/weather/climatechange/climate-action/modelling-ghg-projections.html).
