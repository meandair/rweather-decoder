# rweather-decoder

Decoders of various weather reports.

The decoders are command-line interface (CLI) applications that store decoded reports in JSON files, which are suitable for further machine processing.

## Roadmap

* [x] METAR / SPECI
  * [x] header, wind, visibility, clouds, temperature, dew point, pressure
  * [x] runway visual range, present and recent weather, wind shear, sea
  * [x] TREND
  * [ ] REMARK
* [ ] TAF
* [ ] SYNOP
* [ ] ACARS
* [ ] ISD

## Installation

To use this crate, you need to have Rust and Cargo installed on your machine. To install Rust, visit the official Rust website at https://www.rust-lang.org/learn/get-started and follow the installation instructions for the given operating system. Rust provides an installer that will install both Rust and Cargo.

After, run the following command:

```shell
[filip@fractal ~]$ cargo install rweather-decoder
```

Cargo will download the crate from the Rust package repository and compile it on your system. After the installation is complete, you can start using the CLI decoders.

## Usage

To decode METAR reports, use the `decode-metar` CLI application, see the help:

```shell
[filip@fractal ~]$ decode-metar --help
rweather-decoder 0.2.1
CLI decoder of METAR reports

USAGE:
    decode-metar [FLAGS] [OPTIONS] <input-globs>... <output>

FLAGS:
    -h, --help            Prints help information
    -p, --pretty-print    Enable pretty-printing of output JSON file
    -q, --quiet           Quiet
    -V, --version         Prints version information

OPTIONS:
    -a, --anchor-time <anchor-time>    Anchor time (YYYY-MM-DD) for the plain file format. Specifies a datetime that is
                                       ideally close to that one when the report was actually published. If given, the
                                       decoded METAR day and time will be converted to a full datetime
    -f, --file-format <file-format>    METAR file format (noaa-metar-cycles, plain) [default: noaa-metar-cycles]

ARGS:
    <input-globs>...    Input files (glob patterns separated by space)
    <output>            Output JSON file. Same input reports will be deduplicated
```

The `decode-metar` tool supports right now two METAR file formats:

1. **noaa-metar-cycles** (default) - METAR reports stored in text files downloaded from the NOAA METAR cycles page located at https://tgftp.nws.noaa.gov/data/observations/metar/cycles/.
2. **plain** - METAR reports stored in text files with one report per row.

The decoded METAR reports will be saved to the output JSON file as an array of objects. For further details on the structure of the output, please check the "Examples" section below. You can also refer to documentation available at https://docs.rs/rweather-decoder which includes differences between Rust data types and the JSON output.

## Examples

To check for the latest METAR reports, visit https://tgftp.nws.noaa.gov/data/observations/metar/cycles/. From there you can download a specific file, for example `16Z.TXT` (cycle 16Z), and use the `decode-metar` CLI tool as follows:

```shell
[filip@fractal ~]$ decode-metar -p 16Z.TXT 16Z.json
```

The decoded METAR reports were saved to the JSON file `16Z.json`. The `-p` option enabled pretty-printing of the output for improved readability. Here is an example of a decoded METAR report for the LFBD airport (Bordeaux–Mérignac Airport):

```json
[
  {
    "station_id": "LFBD",
    "observation_time": {
      "value_type": "date_time",
      "value": "2023-05-12T16:00:00Z"
    },
    "is_corrected": false,
    "is_automated": true,
    "wind_from_direction": {
      "value_type": "exact",
      "value": 330.0,
      "units": "degT"
    },
    "wind_from_direction_range": {
      "value_type": "range",
      "value": [
        {
          "value_type": "exact",
          "value": 270.0
        },
        {
          "value_type": "exact",
          "value": 40.0
        }
      ],
      "units": "degT"
    },
    "wind_speed": {
      "value_type": "exact",
      "value": 16.0,
      "units": "kt"
    },
    "wind_gust": {
      "value_type": "exact",
      "value": 32.0,
      "units": "kt"
    },
    "prevailing_visibility": {
      "value_type": "above",
      "value": 10000.0,
      "units": "m"
    },
    "minimum_visibility": {
      "value_type": "exact",
      "value": 600.0,
      "units": "m"
    },
    "directional_visibilites": [],
    "runway_visual_ranges": [
      {
        "runway": "23",
        "visual_range": {
          "value_type": "exact",
          "value": 1100.0,
          "units": "m"
        },
        "trend": "decreasing"
      },
      {
        "runway": "05",
        "visual_range": {
          "value_type": "above",
          "value": 2300.0,
          "units": "m"
        },
        "trend": null
      },
      {
        "runway": "29",
        "visual_range": {
          "value_type": "exact",
          "value": 1800.0,
          "units": "m"
        },
        "trend": "decreasing"
      }
    ],
    "present_weather": [
      {
        "intensity": "heavy",
        "is_in_vicinity": false,
        "descriptors": [
          "thunderstorm"
        ],
        "phenomena": [
          "rain"
        ]
      },
      {
        "intensity": "moderate",
        "is_in_vicinity": false,
        "descriptors": [
          "patches"
        ],
        "phenomena": [
          "fog"
        ]
      }
    ],
    "clouds": [
      {
        "cover": "few",
        "height": {
          "value_type": "exact",
          "value": 2400.0,
          "units": "ft"
        },
        "cloud_type": null
      },
      {
        "cover": "broken",
        "height": {
          "value_type": "exact",
          "value": 3800.0,
          "units": "ft"
        },
        "cloud_type": null
      },
      {
        "cover": "broken",
        "height": {
          "value_type": "exact",
          "value": 4400.0,
          "units": "ft"
        },
        "cloud_type": null
      },
      {
        "cover": null,
        "height": null,
        "cloud_type": "cumulonimbus"
      }
    ],
    "temperature": {
      "value_type": "exact",
      "value": 15.0,
      "units": "degC"
    },
    "dew_point": {
      "value_type": "exact",
      "value": 11.0,
      "units": "degC"
    },
    "pressure": {
      "value_type": "exact",
      "value": 1018.0,
      "units": "hPa"
    },
    "recent_weather": [],
    "wind_shears": [],
    "sea_temperature": null,
    "sea_state": null,
    "wave_height": null,
    "trend_changes": [
      {
        "indicator": "temporary",
        "from_time": null,
        "to_time": null,
        "at_time": null,
        "wind_from_direction": null,
        "wind_from_direction_range": null,
        "wind_speed": null,
        "wind_gust": null,
        "prevailing_visibility": {
          "value_type": "exact",
          "value": 3000.0,
          "units": "m"
        },
        "minimum_visibility": null,
        "directional_visibilites": [],
        "weather": [
          {
            "intensity": "moderate",
            "is_in_vicinity": false,
            "descriptors": [
              "shower"
            ],
            "phenomena": [
              "rain"
            ]
          }
        ],
        "clouds": [
          {
            "cover": "broken",
            "height": {
              "value_type": "exact",
              "value": 1000.0,
              "units": "ft"
            },
            "cloud_type": null
          },
          {
            "cover": "scattered",
            "height": {
              "value_type": "exact",
              "value": 2000.0,
              "units": "ft"
            },
            "cloud_type": "cumulonimbus"
          },
          {
            "cover": "broken",
            "height": {
              "value_type": "exact",
              "value": 3000.0,
              "units": "ft"
            },
            "cloud_type": "towering_cumulus"
          }
        ]
      }
    ],
    "report": "LFBD 121600Z AUTO 33016G32KT 270V040 9999 0600 R23/1100D R05/P2300 R29/1800D +TSRA BCFG FEW024/// BKN038/// BKN044/// //////CB 15/11 Q1018 TEMPO 3000 SHRA BKN010 SCT020CB BKN030TCU"
  }
]
```
