# rweather-decoder

Decoders for various weather reports.

The decoders are binary applications that store decoded reports in JSON files, which are suitable for further machine processing.

## Roadmap

* [x] METAR / SPECI
  * [ ] wind shear
  * [ ] sea state
  * [ ] TREND forecast
  * [ ] REMARK section
* [ ] TAF
* [ ] SYNOP
* [ ] ACARS
* [ ] ISD

## Installation

To use this crate, you need to have Rust and Cargo installed on your machine. To install Rust, visit the official Rust website at https://www.rust-lang.org/learn/get-started and follow the installation instructions for your operating system. Rust provides an installer that will install both Rust and Cargo.

After, you can easily install the decoders by running the following command:

```shell
[filip@fractal ~]$ cargo install rweather-decoder
```

Cargo will download the crate from the Rust package repository and compile it on your system. After the installation is complete, you can start using the command-line decoder tools.

## Usage

To decode METAR reports, use the `decode-metar` binary application, see its help:

```shell
[filip@fractal ~]$ decode-metar -h
rweather-decoder 0.1.2
CLI decoder of METAR reports

USAGE:
    decode-metar [FLAGS] [OPTIONS] <input-globs>... <output>

FLAGS:
    -h, --help            Prints help information
    -p, --pretty-print    Enable pretty-printing of output JSON file
    -q, --quiet           Quiet
    -V, --version         Prints version information

OPTIONS:
    -a, --anchor-time <anchor-time>    Anchor time (YYYY-MM-DD) for the plain file format. Specifies a day close to the
                                       one when the reports were collected. If given, the individual METAR day will be
                                       matched against it to create a proper datetime representation
    -f, --file-format <file-format>    METAR file format (noaa-metar-cycles, plain) [default: noaa-metar-cycles]

ARGS:
    <input-globs>...    Input files (glob patterns separated by space)
    <output>            Output JSON file. Same input reports will be deduplicated
```

The tool supports two METAR file formats:

1. **noaa-metar-cycles** (default) - METAR reports in files downloaded from the NOAA METAR cycles page located at https://tgftp.nws.noaa.gov/data/observations/metar/cycles/.
2. **plain** - METAR reports in files with one report per row.

## Examples

To check for the latest METAR reports, visit https://tgftp.nws.noaa.gov/data/observations/metar/cycles/. You can download a specific file, such as `16Z.TXT` (cycle 16Z), and use the `decode-metar` command-line tool as follows:

```shell
[filip@fractal ~]$ decode-metar -p 16Z.TXT 16Z.json
```

The decoded reports will be saved in the JSON file `16Z.json`. The `-p` option enables pretty-printing of the output JSON file for improved readability. The output file contains an array of decoded reports. Here is an example of a decoded METAR report for the LFBD airport (Bordeaux–Mérignac Airport):

```json
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
    "ceiling": {
      "value_type": "exact",
      "value": 3800.0,
      "units": "ft"
    },
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
    "report": "LFBD 121600Z AUTO 33016G32KT 270V040 9999 0600 R23/1100D R05/P2300 R29/1800D +TSRA BCFG FEW024/// BKN038/// BKN044/// ///CB 15/11 Q1018 TEMPO 3000 SHRA BKN010 SCT020CB BKN030TCU"
  },
```
