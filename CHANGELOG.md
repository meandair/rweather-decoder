# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.1] - XXXX-XX-XX

### Added

- Extended documentation (#13).

### Changed

- CHANGELOG based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
- Visibility of most enums and structs to public.

## [0.2.0] - 2023-12-20

### Added

- Tests for the RVR section (#4).
- Tests for the present and recent weather sections (#5).
- Tests for the cloud and ceiling sections (#6).
- Tests for the temperature section (#7).
- Tests for the pressure section (#8).
- Decoding and tests for the wind shear section (#9).
- Decoding and tests for the sea state section (#10).
- Decoding and tests for the TREND forecast (#14).
- Tests for the datetime guessing (#21).

### Changed

- Make README more readable.
- Standardize Makefile, based on [Standard Targets for Users](https://www.gnu.org/software/make/manual/html_node/Standard-Targets.html).
- Allow multiple descriptors in the weather section.
- Improve performance of CLI tools.

### Removed

- Derived ceiling quantity.

### Fixed

- Bug when CCA-CCZ group does not flag a corrected report.
- A set of several known bugs (#11).
- Crashing decoder due to unwrapping invalid time.
- Crashing decoder when wrongly parsing the `P<xxxx>VP<xxxx>` range.

## [0.1.1] - 2023-05-15

### Added

- Tests for the visibility section (#3).
- Simple documentation for public items.
- Detailed README.

### Changed

- Move tempfile into `[dev-dependencies]`.

### Fixed

- Bug when visibility is left unparsed due to units being separated from the number.

## [0.1.0] - 2023-05-09

### Added

- Decode the main section of a METAR weather report.
- decode-metar binary application.
- Tests for the header section.
- Tests for the plain "day-time" datetime variant (#1).
- Tests for the wind section (#2).
