# CHANGELOG

## v0.2.0
* include tests for the RVR section, #4
* fix bug when CCA-CCZ group does not flag corrected report
* further improve README
* include tests for temperature section, #7
* include tests for pressure section, #8
* allow multiple descriptors in weather section
* include tests for present and recent weather sections, #5
* include tests for cloud and ceiling sections, #6
* standardize `Makefile`
* improve performance of CLI tools
* add decoding and tests for wind shear, #9
* add decoding and tests for sea state, #10
* fix a set of known bugs, #11
* skip rainfall and runway state groups
* fix failing decoder due to unwraping invalid time
* remove derived ceiling value
* parse some missed non-standard groups
* add tests for datetime guessing, #21
* fix failing decoder when parsing `P<xxxx>VP<xxxx>` range
* decode TREND forecast and add tests, #14

## v0.1.1
* move `tempfile` into `[dev-dependencies]`
* document more public items
* include tests for the visibility section, #3
* fix bug when visibility units are separated from the number, e.g. `1 SM`
* extend README

## v0.1.0
* decode the main section of a METAR weather report
* include tests for the header section
* include tests for plain day-time, #1
* include tests for the wind section, #2
* add the `decode-metar` binary application
