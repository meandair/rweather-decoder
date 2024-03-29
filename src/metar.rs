//! Module for decoding METAR reports.
//!
//! The decoding is written based on the following publications:
//! - World Meteorological Organization (2022). Aerodrome reports and forecasts: A Users’ Handbook to the Codes. Available: <https://library.wmo.int/idurl/4/30224>.
//! - World Meteorological Organization (2019). Manual on Codes, Volume I.1 – International Codes. Available: <https://library.wmo.int/idurl/4/35713>.
//! - World Meteorological Organization (2018). Manual on Codes, Volume II – Regional Codes and National Coding Practices. Available: <https://library.wmo.int/idurl/4/35717>.

use std::{ops::{Div, Mul}, str::FromStr};

use anyhow::{anyhow, Error, Result};
use chrono::{NaiveDateTime, NaiveTime, Datelike, Duration};
use chronoutil::RelativeDuration;
use lazy_static::lazy_static;
use regex::Regex;
use serde::{Serialize, Deserialize};

use crate::datetime::{UtcDateTime, UtcDayTime, UtcTime};

lazy_static! {
    static ref WHITESPACE_REPLACE_RE: Regex = Regex::new(r"\s+").unwrap();
    static ref WHITESPACE_REPLACE_OUT: &'static str = " ";

    static ref END_REPLACE_RE: Regex = Regex::new(r"[\s=]*$").unwrap();
    static ref END_REPLACE_OUT: &'static str = " ";

    static ref SECTION_RE: Regex = Regex::new(r"(?x)
        ^(?P<section>NOSIG|TEMPO|BECMG|RMK)
        (?P<end>\s)
    ").unwrap();

    static ref HEADER_RE: Regex = Regex::new(r"(?x)
        ^(?P<station_id>[A-Z][A-Z0-9]{3})
        \s
        (?P<day>\d\d)
        (?P<hour>\d\d)
        (?P<minute>\d\d)\d?Z?
        (\s(?P<corrected>COR|CC[A-Z]))?
        (\s(?P<auto>AUTO))?
        (?P<end>\s)
    ").unwrap();

    static ref WIND_RE: Regex = Regex::new(r"(?x)
        ^E?(?P<direction>\d\d\d|VRB|///)
        (?P<speed>P?\d\d|//)
        (G(?P<gust>P?\d\d|//))?
        (?P<units>KT|MPS)
        (\s(?P<direction_range>\d\d\dV\d\d\d))?
        (?P<end>\s)
    ").unwrap();

    static ref VISIBILITY_RE: Regex = Regex::new(r"(?x)
        ^(?P<prevailing>[MP]?(\d+\s)?\d/\d{1,2}|[MP]?\d{1,5}|////|[CK]AVOK)
        (NDV)?
        \s?
        (?P<units>SM|KM)?
        (\s(?P<minimum>[MP]?\d{1,4}))?
        (?P<directional>(\s[MP]?\d{1,4}[NESW][EW]?)+)?
        (?P<end>\s)
    ").unwrap();

    static ref DIRECTIONAL_VISIBILITY_RE: Regex = Regex::new(r"(?x)
        ^(?P<visibility>[MP]?\d{1,4})
        (?P<direction>[NESW][EW]?)
    ").unwrap();

    static ref RUNWAY_VISUAL_RANGE_RE: Regex = Regex::new(r"(?x)
        ^R(?P<runway>\d\d[A-Z]?)
        /
        (?P<visual_range>[MP]?\d\d\d\d(V[MP]?\d\d\d\d)?)
        (?P<units>FT)?
        /?
        (?P<trend>[UDN])?
        (?P<end>\s)
    ").unwrap();

    static ref PRESENT_WEATHER_RE: Regex = Regex::new(r"(?x)
        ^(?P<intensity>[-\+])?
        (?P<code>(VC|MI|BC|PR|DR|BL|SH|TS|FZ|DZ|RA|SN|SG|PL|GR|GS|UP|BR|FG|FU|VA|DU|SA|HZ|PO|SQ|FC|SS|DS|IC|PY|NSW)+)
        (?P<end>\s)
    ").unwrap();

    static ref CLOUD_RE: Regex = Regex::new(r"(?x)
        ^(?P<cover>CLR|SKC|NSC|NCD|FEW|SCT|BKN|OVC|VV|///)
        (?P<height>\d{1,3}|///)?
        (?P<cloud>AC|ACC|ACSL|AS|CB|CBMAM|CC|CCSL|CI|CS|CU|NS|SC|SCSL|ST|TC?U|///)?
        (?P<end>\s)
    ").unwrap();

    static ref TEMPERATURE_RE: Regex = Regex::new(r"(?x)
        ^(?P<temperature>M?\d{1,2}|//|XX)
        /
        (?P<dew_point>M?\d{1,2}|//|XX)?
        (?P<end>\s)
    ").unwrap();

    static ref PRESSURE_RE: Regex = Regex::new(r"(?x)
        ^(?P<units>A|Q)
        (?P<pressure>\d{3,4}|////)
        (?P<end>\s)
    ").unwrap();

    static ref RECENT_WEATHER_RE: Regex = Regex::new(r"(?x)
        ^RE(?P<intensity>[-\+])?
        (?P<code>(VC|MI|BC|PR|DR|BL|SH|TS|FZ|DZ|RA|SN|SG|PL|GR|GS|UP|BR|FG|FU|VA|DU|SA|HZ|PO|SQ|FC|SS|DS|IC|PY|NSW)+)
        (?P<end>\s)
    ").unwrap();

    static ref WIND_SHEAR_RE: Regex = Regex::new(r"(?x)
        ^WS
        \s
        (?P<runway>R\d\d[A-Z]?|ALL\sRWY)
        (?P<end>\s)
    ").unwrap();

    static ref SEA_RE: Regex = Regex::new(r"(?x)
        ^W(?P<temperature>M?\d{1,2}|//|XX)
        /
        (S(?P<state>\d|/))?
        (H(?P<height>\d{1,3}|///))?
        (?P<end>\s)
    ").unwrap();

    static ref COLOR_RE: Regex = Regex::new(r"(?x)
        ^(BLACK|BLU\+?|GRN|WHT|RED|AMB|YLO)+
        (?P<end>\s)
    ").unwrap();

    static ref RAINFALL_RE: Regex = Regex::new(r"(?x)
        ^RF[\d/]{2}[\./][\d/]/[\d/]{3}[\./][\d/]
        (?P<end>\s)
    ").unwrap();

    static ref RUNWAY_STATE_RE: Regex = Regex::new(r"(?x)
        ^R\d\d[A-Z]?/([\d/]{6}|CLRD[\d/]{2})
        (?P<end>\s)
    ").unwrap();

    static ref TREND_TIME_RE: Regex = Regex::new(r"(?x)
        ^(?P<indicator>FM|TL|AT)
        \s?
        (?P<hour>\d\d)
        (?P<minute>\d\d)Z?
        (?P<end>\s)
    ").unwrap();
}

/// TREND forecast change indicator.
///
/// JSON representation is in lowercase snake case.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Trend {
    /// No significant changes are expected.
    #[default]
    NoSignificantChange,
    /// Expected temporary fluctuations in the meteorological conditions.
    Temporary,
    /// Expected changes which reach or pass specified values.
    Becoming,
}

impl FromStr for Trend {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "NOSIG" => Ok(Trend::NoSignificantChange),
            "TEMPO" => Ok(Trend::Temporary),
            "BECMG" => Ok(Trend::Becoming),
            _ => Err(anyhow!("Invalid trend, given {}", s))
        }
    }
}

#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Section {
    Main,
    Trend(Trend),
    Remark,
}

impl FromStr for Section {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "RMK" => Ok(Section::Remark),
            s => match Trend::from_str(s) {
                Ok(trend) => Ok(Section::Trend(trend)),
                Err(_) => Err(anyhow!("Invalid section, given {}", s))
            }
        }
    }
}

fn handle_section(text: &str) -> Option<(Section, usize)> {
    SECTION_RE.captures(text)
        .map(|capture| {
            let section = Section::from_str(&capture["section"]).unwrap();
            let end = capture.name("end").unwrap().end();

            (section, end)
        })
}

/// METAR date and time combinations.
///
/// JSON representation is adjacently tagged and in lowercase snake case. Example:
/// ```json
/// {
///     "value_type": "date_time",
///     "value": "2023-12-27T08:30:00Z"
/// }
/// ```
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "value_type", content = "value", rename_all = "snake_case")]
pub enum MetarTime {
    /// Date and time.
    DateTime(UtcDateTime),
    /// Day and time.
    DayTime(UtcDayTime),
    /// Time only.
    Time(UtcTime),
}

impl MetarTime {
    /// Converts any [MetarTime] into [MetarTime::DateTime].
    ///
    /// Using `anchor_time`, any [MetarTime] will be converted to a [MetarTime::DateTime] that is nearest
    /// to the specified `anchor_time` while preserving all the datetime information in the input [MetarTime].
    /// This conversion correctly handles months with different number of days and also leap years.
    pub fn to_date_time(&self, anchor_time: NaiveDateTime) -> MetarTime {
        match self {
            MetarTime::DateTime(utc_dt) => MetarTime::DateTime(*utc_dt),
            MetarTime::DayTime(utc_d_t) => {
                let first_guess_opt = anchor_time.date().with_day(utc_d_t.0).map(|nd| nd.and_time(utc_d_t.1));
                let second_guess_opt = (anchor_time + RelativeDuration::months(-1)).date().with_day(utc_d_t.0).map(|nd| nd.and_time(utc_d_t.1));
                let third_guess_opt = (anchor_time + RelativeDuration::months(1)).date().with_day(utc_d_t.0).map(|nd| nd.and_time(utc_d_t.1));

                let mut final_guess_opt = None;
                let mut final_delta = i64::MAX;

                for guess_opt in [first_guess_opt, second_guess_opt, third_guess_opt] {
                    if let Some(guess) = guess_opt {
                        let delta = guess.signed_duration_since(anchor_time).num_seconds().abs();
                        if delta < final_delta {
                            final_guess_opt = guess_opt;
                            final_delta = delta;
                        }
                    }
                }

                match final_guess_opt {
                    Some(final_guess) => MetarTime::DateTime(UtcDateTime(final_guess)),
                    None => panic!("{}", format!("Date guessing failed, given time {:?} and anchor time {}", self, anchor_time))
                }
            },
            MetarTime::Time(utc_t) => {
                let first_guess = anchor_time.date().and_time(utc_t.0);
                let second_guess = first_guess + Duration::days(-1);
                let third_guess = first_guess + Duration::days(1);

                let mut final_guess = first_guess;
                let mut final_delta = final_guess.signed_duration_since(anchor_time).num_seconds().abs();

                for guess in [second_guess, third_guess] {
                    let delta = guess.signed_duration_since(anchor_time).num_seconds().abs();
                    if delta < final_delta {
                        final_guess = guess;
                        final_delta = delta;
                    }
                }

                MetarTime::DateTime(UtcDateTime(final_guess))
            },
        }
    }
}

/// Identification groups.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Header {
    /// ICAO airport code.
    pub station_id: Option<String>,
    /// Observation time of the report.
    pub observation_time: Option<MetarTime>,
    /// Flag if the report is corrected.
    pub is_corrected: Option<bool>,
    /// Flag if the report comes from a fully automated observation.
    pub is_automated: Option<bool>,
}

impl Header {
    fn is_empty(&self) -> bool {
        self.station_id.is_none() && self.observation_time.is_none() && self.is_corrected.is_none() && self.is_automated.is_none()
    }
}

fn handle_header(text: &str, anchor_time: Option<NaiveDateTime>) -> Option<(Header, usize)> {
    HEADER_RE.captures(text)
        .map(|capture| {
            let station_id = Some(capture["station_id"].to_string());

            let day = capture["day"].parse().unwrap();
            let hour = capture["hour"].parse().unwrap();
            let minute = capture["minute"].parse().unwrap();

            let naive_time = NaiveTime::from_hms_opt(hour, minute, 0);
            let mut time = naive_time.map(|nt| MetarTime::DayTime(UtcDayTime(day, nt)));

            if let Some(at) = anchor_time {
                time = time.map(|t| t.to_date_time(at));
            }

            let is_corrected = Some(capture.name("corrected").is_some());

            let is_automated = Some(capture.name("auto").is_some());

            let end = capture.name("end").unwrap().end();

            let header = Header { station_id, observation_time: time, is_corrected, is_automated };

            (header, end)
        })
}

/// Unit of a physical quantity.
///
/// JSON representation is using common symbols.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Unit {
    /// True degree.
    ///
    /// JSON representation:
    /// ```json
    /// "degT"
    /// ```
    #[serde(rename = "degT")]
    DegreeTrue,
    /// Knot.
    ///
    /// JSON representation:
    /// ```json
    /// "kt"
    /// ```
    #[serde(rename = "kt")]
    Knot,
    /// Metre per second.
    ///
    /// JSON representation:
    /// ```json
    /// "m/s"
    /// ```
    #[serde(rename = "m/s")]
    MetrePerSecond,
    /// Kilometre.
    ///
    /// JSON representation:
    /// ```json
    /// "km"
    /// ```
    #[serde(rename = "km")]
    KiloMetre,
    /// Metre.
    ///
    /// JSON representation:
    /// ```json
    /// "m"
    /// ```
    #[serde(rename = "m")]
    Metre,
    /// Statute mile.
    ///
    /// JSON representation:
    /// ```json
    /// "mi"
    /// ```
    #[serde(rename = "mi")]
    StatuteMile,
    /// Foot.
    ///
    /// JSON representation:
    /// ```json
    /// "ft"
    /// ```
    #[serde(rename = "ft")]
    Foot,
    /// Degree Celsius.
    ///
    /// JSON representation:
    /// ```json
    /// "degC"
    /// ```
    #[serde(rename = "degC")]
    DegreeCelsius,
    /// Hectopascal.
    ///
    /// JSON representation:
    /// ```json
    /// "hPa"
    /// ```
    #[serde(rename = "hPa")]
    HectoPascal,
    /// Inch of mercury.
    ///
    /// JSON representation:
    /// ```json
    /// "inHg"
    /// ```
    #[serde(rename = "inHg")]
    InchOfMercury,
}

impl FromStr for Unit {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "KT" => Ok(Unit::Knot),
            "MPS" => Ok(Unit::MetrePerSecond),
            "KM" => Ok(Unit::KiloMetre),
            "SM" => Ok(Unit::StatuteMile),
            "FT" => Ok(Unit::Foot),
            "Q" => Ok(Unit::HectoPascal),
            "A" => Ok(Unit::InchOfMercury),
            _ => Err(anyhow!("Invalid units, given {}", s))
        }
    }
}

fn parse_value(s: &str) -> Result<f32> {
    if s.contains(' ') && s.contains('/') {
        let mut split_space = s.split(' ');
        let number: f32 = split_space.next().unwrap().parse()?;

        let mut split_slash = split_space.next().unwrap().split('/');
        let numerator: f32 = split_slash.next().unwrap().parse()?;
        let denominator: f32 = split_slash.next().unwrap().parse()?;

        Ok(number + numerator / denominator)
    } else if s.contains('/') {
        let mut split = s.split('/');
        let numerator: f32 = split.next().unwrap().parse()?;
        let denominator: f32 = split.next().unwrap().parse()?;

        Ok(numerator / denominator)
    } else {
        Ok(s.parse()?)
    }
}

/// Value in range variants.
///
/// JSON representation is adjacently tagged and in lowercase snake case. Example:
/// ```json
/// {
///     "value_type": "above",
///     "value": 3.5
/// }
/// ```
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(tag = "value_type", content = "value", rename_all = "snake_case")]
pub enum ValueInRange {
    /// Above specified number.
    Above(f32),
    /// Below specified number.
    Below(f32),
    /// Same as specified number.
    Exact(f32),
}

impl FromStr for ValueInRange {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Some(stripped) = s.strip_prefix('P') {
            let value = parse_value(stripped).unwrap();
            Ok(ValueInRange::Above(value))
        } else if let Some(stripped) = s.strip_prefix('M') {
            let value = parse_value(stripped).unwrap();
            Ok(ValueInRange::Below(value))
        } else {
            let value = parse_value(s).unwrap();
            Ok(ValueInRange::Exact(value))
        }
    }
}

impl Div<f32> for ValueInRange {
    type Output = ValueInRange;

    fn div(self, rhs: f32) -> Self::Output {
        match self {
            ValueInRange::Above(x) => ValueInRange::Above(x / rhs),
            ValueInRange::Below(x) => ValueInRange::Below(x / rhs),
            ValueInRange::Exact(x) => ValueInRange::Exact(x / rhs),
        }
    }
}

impl Mul<f32> for ValueInRange {
    type Output = ValueInRange;

    fn mul(self, rhs: f32) -> Self::Output {
        match self {
            ValueInRange::Above(x) => ValueInRange::Above(x * rhs),
            ValueInRange::Below(x) => ValueInRange::Below(x * rhs),
            ValueInRange::Exact(x) => ValueInRange::Exact(x * rhs),
        }
    }
}

/// Value variants.
///
/// JSON representation is adjacently tagged and in lowercase snake case. Example:
/// ```json
/// {
///     "value_type": "below",
///     "value": 3.5
/// }
/// ```
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(tag = "value_type", content = "value", rename_all = "snake_case")]
pub enum Value {
    /// Variable number.
    Variable,
    /// Above specified number.
    Above(f32),
    /// Below specified number.
    Below(f32),
    /// Between specified [`ValueInRange`] values.
    Range(ValueInRange, ValueInRange),
    /// Same as specified number.
    Exact(f32),
}

impl FromStr for Value {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s == "VRB" {
            Ok(Value::Variable)
        } else if s.contains('V') {
            let mut split = s.split('V');
            let value1 = ValueInRange::from_str(split.next().unwrap()).unwrap();
            let value2 = ValueInRange::from_str(split.next().unwrap()).unwrap();
            Ok(Value::Range(value1, value2))
        } else if let Some(stripped) = s.strip_prefix('P') {
            let value = parse_value(stripped).unwrap();
            Ok(Value::Above(value))
        } else if let Some(stripped) = s.strip_prefix('M') {
            let value = parse_value(stripped).unwrap();
            Ok(Value::Below(value))
        } else {
            let value = parse_value(s).unwrap();
            Ok(Value::Exact(value))
        }
    }
}

impl Div<f32> for Value {
    type Output = Value;

    fn div(self, rhs: f32) -> Self::Output {
        match self {
            Value::Variable => Value::Variable,
            Value::Above(x) => Value::Above(x / rhs),
            Value::Below(x) => Value::Below(x / rhs),
            Value::Range(x, y) => Value::Range(x / rhs, y / rhs),
            Value::Exact(x) => Value::Exact(x / rhs),
        }
    }
}

impl Mul<f32> for Value {
    type Output = Value;

    fn mul(self, rhs: f32) -> Self::Output {
        match self {
            Value::Variable => Value::Variable,
            Value::Above(x) => Value::Above(x * rhs),
            Value::Below(x) => Value::Below(x * rhs),
            Value::Range(x, y) => Value::Range(x * rhs, y * rhs),
            Value::Exact(x) => Value::Exact(x * rhs),
        }
    }
}

/// Physical quantity.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Quantity {
    /// Value.
    ///
    /// JSON representation is flattened once.
    #[serde(flatten)]
    pub value: Value,
    pub units: Unit,
}

impl Quantity {
    fn new(value: Value, units: Unit) -> Quantity {
        Quantity { value, units }
    }

    fn new_opt(value: Option<Value>, units: Unit) -> Option<Quantity> {
        value.map(|v| Quantity { value: v, units })
    }
}

/// Surface wind groups.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct Wind {
    /// Wind direction. Reported by the direction from which the wind originates.
    pub wind_from_direction: Option<Quantity>,
    /// Range of wind directions if they vary significantly.
    /// Reported by the direction from which the wind originates.
    pub wind_from_direction_range: Option<Quantity>,
    pub wind_speed: Option<Quantity>,
    pub wind_gust: Option<Quantity>,
}

impl Wind {
    fn is_empty(&self) -> bool {
        self.wind_from_direction.is_none() && self.wind_from_direction_range.is_none() && self.wind_speed.is_none() && self.wind_gust.is_none()
    }
}

fn handle_wind(text: &str) -> Option<(Wind, usize)> {
    WIND_RE.captures(text)
        .map(|capture| {
            let mut from_direction_value = match &capture["direction"] {
                "///" => None,
                s => Some(Value::from_str(s).unwrap()),
            };

            if &capture["direction"] == "000" && &capture["speed"] == "00" {
                // calm wind has no direction
                from_direction_value = None;
            }

            let speed_value = match &capture["speed"] {
                "//" => None,
                s => Some(Value::from_str(s).unwrap()),
            };

            let gust_value = capture.name("gust").and_then(|c| match c.as_str() {
                "//" => None,
                s => Some(Value::from_str(s).unwrap()),
            });

            let units = Unit::from_str(&capture["units"]).unwrap();

            let from_direction_range_value = capture.name("direction_range")
                .map(|s| Value::from_str(s.as_str()).unwrap());

            let wind_from_direction = Quantity::new_opt(from_direction_value, Unit::DegreeTrue);
            let wind_from_direction_range = Quantity::new_opt(from_direction_range_value, Unit::DegreeTrue);
            let wind_speed = Quantity::new_opt(speed_value, units);
            let wind_gust = Quantity::new_opt(gust_value, units);

            let end = capture.name("end").unwrap().end();

            let wind = Wind { wind_from_direction, wind_from_direction_range, wind_speed, wind_gust };

            (wind, end)
        })
}

/// Direction octant.
///
/// JSON representation is in lowercase snake case.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DirectionOctant {
    North,
    NorthEast,
    East,
    SouthEast,
    South,
    SouthWest,
    West,
    NorthWest,
}

impl FromStr for DirectionOctant {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "N" => Ok(DirectionOctant::North),
            "NE" => Ok(DirectionOctant::NorthEast),
            "E" => Ok(DirectionOctant::East),
            "SE" => Ok(DirectionOctant::SouthEast),
            "S" => Ok(DirectionOctant::South),
            "SW" => Ok(DirectionOctant::SouthWest),
            "W" => Ok(DirectionOctant::West),
            "NW" => Ok(DirectionOctant::NorthWest),
            _ => Err(anyhow!("Invalid direction octant, given {}", s))
        }
    }
}

/// Directional visibility.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct DirectionalVisibility {
    pub visibility: Quantity,
    pub direction: DirectionOctant,
}

/// Visibility groups.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Visibility {
    pub prevailing_visibility: Option<Quantity>,
    pub minimum_visibility: Option<Quantity>,
    pub directional_visibilites: Vec<DirectionalVisibility>,
}

impl Visibility {
    fn is_empty(&self) -> bool {
        self.prevailing_visibility.is_none() && self.minimum_visibility.is_none() && self.directional_visibilites.is_empty()
    }
}

fn handle_visibility(text: &str) -> Option<(Visibility, bool, usize)> {
    VISIBILITY_RE.captures(text)
        .map(|capture| {
            let mut is_cavok = false;

            let mut prevailing_visibility_value = match &capture["prevailing"] {
                "////" => None,
                "CAVOK" | "KAVOK" => {
                    is_cavok = true;
                    Some(Value::Above(10000.0))
                },
                s => Some(Value::from_str(s).unwrap()),
            };

            let units = capture.name("units")
                .map(|c| Unit::from_str(c.as_str()).unwrap())
                .unwrap_or(Unit::Metre);

            if prevailing_visibility_value == Some(Value::Exact(9999.0)) && units == Unit::Metre {
                prevailing_visibility_value = Some(Value::Above(10000.0));
            }

            let minimum_visibility_value = capture.name("minimum").map(|c| Value::from_str(c.as_str()).unwrap());

            let directional_visibilites = capture.name("directional")
                .map(|c| c.as_str().split(' ')
                    .map(|group| DIRECTIONAL_VISIBILITY_RE.captures(group))
                    .filter(|capture| capture.is_some())
                    .map(|capture| DirectionalVisibility {
                        visibility: Quantity::new(Value::from_str(&capture.as_ref().unwrap()["visibility"]).unwrap(), units),
                        direction: DirectionOctant::from_str(&capture.unwrap()["direction"]).unwrap(),
                    })
                    .collect::<Vec<_>>())
                .unwrap_or_default();

            let prevailing_visibility = Quantity::new_opt(prevailing_visibility_value, units);
            let minimum_visibility = Quantity::new_opt(minimum_visibility_value, units);

            let end = capture.name("end").unwrap().end();

            let visibility = Visibility { prevailing_visibility, minimum_visibility, directional_visibilites };

            (visibility, is_cavok, end)
        })
}

/// Runway visual range (RVR) trend.
///
/// JSON representation is in lowercase snake case.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunwayVisualRangeTrend {
    Increasing,
    Decreasing,
    NoChange,
}

impl FromStr for RunwayVisualRangeTrend {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "U" => Ok(RunwayVisualRangeTrend::Increasing),
            "D" => Ok(RunwayVisualRangeTrend::Decreasing),
            "N" => Ok(RunwayVisualRangeTrend::NoChange),
            _ => Err(anyhow!("Invalid runway visual range trend, given {}", s))
        }
    }
}

/// Runway visual range (RVR).
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RunwayVisualRange {
    pub runway: String,
    pub visual_range: Quantity,
    pub trend: Option<RunwayVisualRangeTrend>,
}

fn handle_runway_visual_range(text: &str) -> Option<(RunwayVisualRange, usize)> {
    RUNWAY_VISUAL_RANGE_RE.captures(text)
        .map(|capture| {
            let runway = capture["runway"].to_string();

            let visual_range_value = Value::from_str(&capture["visual_range"]).unwrap();

            let units = capture.name("units")
                .map(|c| Unit::from_str(c.as_str()).unwrap())
                .unwrap_or(Unit::Metre);

            let trend = capture.name("trend")
                .map(|c| RunwayVisualRangeTrend::from_str(c.as_str()).unwrap());

            let visual_range = Quantity::new(visual_range_value, units);

            let end = capture.name("end").unwrap().end();

            let rvr = RunwayVisualRange { runway, visual_range, trend };

            (rvr, end)
        })
}

/// Weather intensity.
///
/// JSON representation is in lowercase snake case.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WeatherIntensity {
    Light,
    Moderate,
    Heavy,
}

impl FromStr for WeatherIntensity {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "-" => Ok(WeatherIntensity::Light),
            "+" => Ok(WeatherIntensity::Heavy),
            _ => Err(anyhow!("Invalid weather intensity, given {}", s))
        }
    }
}

/// Weather descriptor.
///
/// JSON representation is in lowercase snake case.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WeatherDescriptor {
    Shallow,
    Patches,
    Partial,
    LowDrifting,
    Blowing,
    Shower,
    Thunderstorm,
    Freezing,
}

impl FromStr for WeatherDescriptor {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "MI" => Ok(WeatherDescriptor::Shallow),
            "BC" => Ok(WeatherDescriptor::Patches),
            "PR" => Ok(WeatherDescriptor::Partial),
            "DR" => Ok(WeatherDescriptor::LowDrifting),
            "BL" => Ok(WeatherDescriptor::Blowing),
            "SH" => Ok(WeatherDescriptor::Shower),
            "TS" => Ok(WeatherDescriptor::Thunderstorm),
            "FZ" => Ok(WeatherDescriptor::Freezing),
            _ => Err(anyhow!("Invalid weather descriptor, given {}", s))
        }
    }
}

/// Weather phenomena.
///
/// JSON representation is in lowercase snake case.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WeatherPhenomena {
    Drizzle,
    Rain,
    Snow,
    SnowGrains,
    IcePellets,
    Hail,
    SnowPellets,
    UnknownPrecipitation,
    Mist,
    Fog,
    Smoke,
    VolcanicAsh,
    Dust,
    Sand,
    Haze,
    DustWhirls,
    Squalls,
    FunnelCloud,
    Sandstorm,
    Duststorm,
    IceCrystals,
    Spray,
    NilSignificantWeather,
}

impl FromStr for WeatherPhenomena {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "DZ" => Ok(WeatherPhenomena::Drizzle),
            "RA" => Ok(WeatherPhenomena::Rain),
            "SN" => Ok(WeatherPhenomena::Snow),
            "SG" => Ok(WeatherPhenomena::SnowGrains),
            "PL" => Ok(WeatherPhenomena::IcePellets),
            "GR" => Ok(WeatherPhenomena::Hail),
            "GS" => Ok(WeatherPhenomena::SnowPellets),
            "UP" => Ok(WeatherPhenomena::UnknownPrecipitation),
            "BR" => Ok(WeatherPhenomena::Mist),
            "FG" => Ok(WeatherPhenomena::Fog),
            "FU" => Ok(WeatherPhenomena::Smoke),
            "VA" => Ok(WeatherPhenomena::VolcanicAsh),
            "DU" => Ok(WeatherPhenomena::Dust),
            "SA" => Ok(WeatherPhenomena::Sand),
            "HZ" => Ok(WeatherPhenomena::Haze),
            "PO" => Ok(WeatherPhenomena::DustWhirls),
            "SQ" => Ok(WeatherPhenomena::Squalls),
            "FC" => Ok(WeatherPhenomena::FunnelCloud),
            "SS" => Ok(WeatherPhenomena::Sandstorm),
            "DS" => Ok(WeatherPhenomena::Duststorm),
            "IC" => Ok(WeatherPhenomena::IceCrystals),
            "PY" => Ok(WeatherPhenomena::Spray),
            "NSW" => Ok(WeatherPhenomena::NilSignificantWeather),
            _ => Err(anyhow!("Invalid weather phenomena, given {}", s))
        }
    }
}

/// Weather condition.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WeatherCondition {
    pub intensity: WeatherIntensity,
    /// Flag if the specified weather condition occurs only in the vicinity
    /// of the aerodrome but not at/above the aerodrome.
    pub is_in_vicinity: bool,
    pub descriptors: Vec<WeatherDescriptor>,
    pub phenomena: Vec<WeatherPhenomena>,
}

fn handle_weather(weather_re: &Regex, text: &str) -> Option<(WeatherCondition, usize)> {
    weather_re.captures(text)
        .map(|capture| {
            let intensity = capture.name("intensity")
                .map(|c| WeatherIntensity::from_str(c.as_str()).unwrap())
                .unwrap_or(WeatherIntensity::Moderate);

            let groups = if &capture["code"] == "NSW" {
                vec!["NSW".to_string()]
            } else {
                capture["code"].chars()
                    .collect::<Vec<_>>()
                    .chunks(2)
                    .map(String::from_iter)
                    .collect::<Vec<_>>()
            };

            let mut is_in_vicinity = false;
            let mut descriptors = Vec::new();
            let mut phenomena = Vec::new();

            for group in groups.iter() {
                if group == "VC" {
                    is_in_vicinity = true;
                } else if let Ok(wd) = WeatherDescriptor::from_str(group) {
                    descriptors.push(wd);
                } else if let Ok(wp) = WeatherPhenomena::from_str(group) {
                    phenomena.push(wp);
                }
            }

            let end = capture.name("end").unwrap().end();

            let weather = WeatherCondition { intensity, is_in_vicinity, descriptors, phenomena };

            (weather, end)
        })
}

fn handle_present_weather(text: &str) -> Option<(WeatherCondition, usize)> {
    handle_weather(&PRESENT_WEATHER_RE, text)
}

fn handle_recent_weather(text: &str) -> Option<(WeatherCondition, usize)> {
    handle_weather(&RECENT_WEATHER_RE, text)
}

/// Cloud cover.
///
/// JSON representation is in lowercase snake case.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CloudCover {
    Clear,
    SkyClear,
    NilSignificantCloud,
    NoCloudDetected,
    Few,
    Scattered,
    Broken,
    Overcast,
    /// Obscured sky but vertical visibility is available.
    VerticalVisibility,
    /// No cloud of operational significance in CAVOK conditions.
    CeilingOk,
}

impl FromStr for CloudCover {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "CLR" => Ok(CloudCover::Clear),
            "SKC" => Ok(CloudCover::SkyClear),
            "NSC" => Ok(CloudCover::NilSignificantCloud),
            "NCD" => Ok(CloudCover::NoCloudDetected),
            "FEW" => Ok(CloudCover::Few),
            "SCT" => Ok(CloudCover::Scattered),
            "BKN" => Ok(CloudCover::Broken),
            "OVC" => Ok(CloudCover::Overcast),
            "VV" => Ok(CloudCover::VerticalVisibility),
            _ => Err(anyhow!("Invalid cloud cover, given {}", s))
        }
    }
}

/// Cloud type.
///
/// JSON representation is in lowercase snake case.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CloudType {
    Altocumulus,
    AltocumulusCastellanus,
    AltocumulusLenticularis,
    Altostratus,
    Cumulonimbus,
    CumulonimbusMammatus,
    Cirrocumulus,
    CirrocumulusLenticularis,
    Cirrus,
    Cirrostratus,
    Cumulus,
    Nimbostratus,
    Stratocumulus,
    StratocumulusLenticularis,
    Stratus,
    ToweringCumulus,
}

impl FromStr for CloudType {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "AC" => Ok(CloudType::Altocumulus),
            "ACC" => Ok(CloudType::AltocumulusCastellanus),
            "ACSL" => Ok(CloudType::AltocumulusLenticularis),
            "AS" => Ok(CloudType::Altostratus),
            "CB" => Ok(CloudType::Cumulonimbus),
            "CBMAM" => Ok(CloudType::CumulonimbusMammatus),
            "CC" => Ok(CloudType::Cirrocumulus),
            "CCSL" => Ok(CloudType::CirrocumulusLenticularis),
            "CI" => Ok(CloudType::Cirrus),
            "CS" => Ok(CloudType::Cirrostratus),
            "CU" => Ok(CloudType::Cumulus),
            "NS" => Ok(CloudType::Nimbostratus),
            "SC" => Ok(CloudType::Stratocumulus),
            "SCSL" => Ok(CloudType::StratocumulusLenticularis),
            "ST" => Ok(CloudType::Stratus),
            "TCU" | "TU" => Ok(CloudType::ToweringCumulus),
            _ => Err(anyhow!("Invalid cloud type, given {s}"))
        }
    }
}

/// Cloud layer.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CloudLayer {
    pub cover: Option<CloudCover>,
    /// Height above the ground level (AGL).
    pub height: Option<Quantity>,
    pub cloud_type: Option<CloudType>,
}

impl CloudLayer {
    fn is_empty(&self) -> bool {
        self.cover.is_none() && self.height.is_none() && self.cloud_type.is_none()
    }
}

fn handle_cloud_layer(text: &str) -> Option<(CloudLayer, usize)> {
    CLOUD_RE.captures(text)
        .map(|capture| {
            let cover = match &capture["cover"] {
                "///" => None,
                s => Some(CloudCover::from_str(s).unwrap()),
            };

            let height_value = capture.name("height").and_then(|c| match c.as_str() {
                "///" => None,
                s => Some(Value::from_str(s).unwrap() * 100.0),
            });

            let cloud_type = capture.name("cloud").and_then(|c| match c.as_str() {
                "///" => None,
                s => Some(CloudType::from_str(s).unwrap()),
            });

            let height = Quantity::new_opt(height_value, Unit::Foot);

            let end = capture.name("end").unwrap().end();

            let cloud_layer = CloudLayer { cover, height, cloud_type };

            (cloud_layer, end)
        })
}

/// Temperature groups.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct Temperature {
    pub temperature: Option<Quantity>,
    pub dew_point: Option<Quantity>,
}

impl Temperature {
    fn is_empty(&self) -> bool {
        self.temperature.is_none() && self.dew_point.is_none()
    }
}

fn handle_temperature(text: &str) -> Option<(Temperature, usize)> {
    TEMPERATURE_RE.captures(text)
        .map(|capture| {
            let temperature_value = match &capture["temperature"] {
                "//" | "XX" => None,
                s => Some(Value::from_str(&s.replace('M', "-")).unwrap()),
            };

            let dew_point_value = capture.name("dew_point").and_then(|c| match c.as_str() {
                "//" | "XX" => None,
                s => Some(Value::from_str(&s.replace('M', "-")).unwrap()),
            });

            let temperature = Quantity::new_opt(temperature_value, Unit::DegreeCelsius);
            let dew_point = Quantity::new_opt(dew_point_value, Unit::DegreeCelsius);

            let end = capture.name("end").unwrap().end();

            let temperature = Temperature { temperature, dew_point };

            (temperature, end)
        })
}

/// Pressure group.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct Pressure {
    pub pressure: Option<Quantity>,
}

impl Pressure {
    fn is_empty(&self) -> bool {
        self.pressure.is_none()
    }
}

fn handle_pressure(text: &str) -> Option<(Pressure, usize)> {
    PRESSURE_RE.captures(text)
        .map(|capture| {
            let mut pressure_value = match &capture["pressure"] {
                "////" => None,
                s => Some(Value::from_str(s).unwrap()),
            };

            let units = Unit::from_str(&capture["units"]).unwrap();

            if units == Unit::InchOfMercury {
                pressure_value = pressure_value.map(|p| p / 100.0)
            }

            let pressure = Quantity::new_opt(pressure_value, units);

            let end = capture.name("end").unwrap().end();

            let pressure = Pressure { pressure };

            (pressure, end)
        })
}

/// Wind shear group.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WindShear {
    pub runway: String,
}

fn handle_wind_shear(text: &str) -> Option<(WindShear, usize)> {
    WIND_SHEAR_RE.captures(text)
        .map(|capture| {
            let runway = match &capture["runway"] {
                "ALL RWY" => "all".to_string(),
                s => s[1..].to_string(),
            };

            let end = capture.name("end").unwrap().end();

            let ws = WindShear { runway };

            (ws, end)
        })
}

/// Sea state from WMO Code Table 3700.
///
/// JSON representation is in lowercase snake case.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SeaState {
    Glassy,
    Rippled,
    Smooth,
    Slight,
    Moderate,
    Rough,
    VeryRough,
    High,
    VeryHigh,
    Phenomenal,
}

impl FromStr for SeaState {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "0" => Ok(SeaState::Glassy),
            "1" => Ok(SeaState::Rippled),
            "2" => Ok(SeaState::Smooth),
            "3" => Ok(SeaState::Slight),
            "4" => Ok(SeaState::Moderate),
            "5" => Ok(SeaState::Rough),
            "6" => Ok(SeaState::VeryRough),
            "7" => Ok(SeaState::High),
            "8" => Ok(SeaState::VeryHigh),
            "9" => Ok(SeaState::Phenomenal),
            _ => Err(anyhow!("Invalid sea state, given {}", s))
        }
    }
}

/// Sea groups.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct Sea {
    pub sea_temperature: Option<Quantity>,
    pub sea_state: Option<SeaState>,
    pub wave_height: Option<Quantity>,
}

impl Sea {
    fn is_empty(&self) -> bool {
        self.sea_temperature.is_none() && self.sea_state.is_none() && self.wave_height.is_none()
    }
}

fn handle_sea(text: &str) -> Option<(Sea, usize)> {
    SEA_RE.captures(text)
        .map(|capture| {
            let temperature_value = match &capture["temperature"] {
                "//" | "XX" => None,
                s => Some(Value::from_str(&s.replace('M', "-")).unwrap()),
            };

            let sea_state = capture.name("state").and_then(|c| match c.as_str() {
                "/" => None,
                s => Some(SeaState::from_str(s).unwrap()),
            });

            let height_value = capture.name("height").and_then(|c| match c.as_str() {
                "///" => None,
                s => Some(Value::from_str(s).unwrap() / 10.0),
            });

            let sea_temperature = Quantity::new_opt(temperature_value, Unit::DegreeCelsius);
            let wave_height = Quantity::new_opt(height_value, Unit::Metre);

            let end = capture.name("end").unwrap().end();

            let sea = Sea { sea_temperature, sea_state, wave_height };

            (sea, end)
        })
}

fn handle_color(text: &str) -> Option<usize> {
    COLOR_RE.captures(text)
        .map(|capture| {
            capture.name("end").unwrap().end()
        })
}

fn handle_rainfall(text: &str) -> Option<usize> {
    RAINFALL_RE.captures(text)
        .map(|capture| {
            capture.name("end").unwrap().end()
        })
}

fn handle_runway_state(text: &str) -> Option<usize> {
    RUNWAY_STATE_RE.captures(text)
        .map(|capture| {
            capture.name("end").unwrap().end()
        })
}

#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum TrendTimeIndicator {
    From,
    Until,
    At,
}

impl FromStr for TrendTimeIndicator {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "FM" => Ok(TrendTimeIndicator::From),
            "TL" => Ok(TrendTimeIndicator::Until),
            "AT" => Ok(TrendTimeIndicator::At),
            _ => Err(anyhow!("Invalid trend time indicator, given {}", s))
        }
    }
}

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct TrendTime {
    indicator: TrendTimeIndicator,
    time: Option<MetarTime>,
}

fn handle_trend_time(text: &str, anchor_time: Option<NaiveDateTime>) -> Option<(TrendTime, usize)> {
    TREND_TIME_RE.captures(text)
        .map(|capture| {
            let indicator = TrendTimeIndicator::from_str(&capture["indicator"]).unwrap();
            let mut hour = capture["hour"].parse().unwrap();
            let minute = capture["minute"].parse().unwrap();

            if hour == 24 {
                hour = 0;
            }

            let naive_time = NaiveTime::from_hms_opt(hour, minute, 0);
            let mut time = naive_time.map(|nt| MetarTime::Time(UtcTime(nt)));

            if let Some(at) = anchor_time {
                time = time.map(|t| t.to_date_time(at));
            }

            let end = capture.name("end").unwrap().end();

            let trend_time = TrendTime { indicator, time };

            (trend_time, end)
        })
}

/// Significant changes in the meteorological conditions in the TREND forecast.
///
/// Only elements for which a significant change is expected are [Option::Some].
#[non_exhaustive]
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct TrendChange {
    pub indicator: Trend,
    pub from_time: Option<MetarTime>,
    pub to_time: Option<MetarTime>,
    pub at_time: Option<MetarTime>,
    /// Surface wind groups.
    ///
    /// JSON representation is flattened once.
    #[serde(flatten)]
    pub wind: Wind,
    /// Visibility groups.
    ///
    /// JSON representation is flattened once.
    #[serde(flatten)]
    pub visibility: Visibility,
    pub weather: Vec<WeatherCondition>,
    pub clouds: Vec<CloudLayer>,
}

/// Decoded METAR report.
#[non_exhaustive]
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Metar {
    /// Identification groups.
    ///
    /// JSON representation is flattened once.
    #[serde(flatten)]
    pub header: Header,
    /// Surface wind groups.
    ///
    /// JSON representation is flattened once.
    #[serde(flatten)]
    pub wind: Wind,
    /// Visibility groups.
    ///
    /// JSON representation is flattened once.
    #[serde(flatten)]
    pub visibility: Visibility,
    pub runway_visual_ranges: Vec<RunwayVisualRange>,
    pub present_weather: Vec<WeatherCondition>,
    pub clouds: Vec<CloudLayer>,
    /// Temperature groups.
    ///
    /// JSON representation is flattened once.
    #[serde(flatten)]
    pub temperature: Temperature,
    /// Pressure group.
    ///
    /// JSON representation is flattened once.
    #[serde(flatten)]
    pub pressure: Pressure,
    pub recent_weather: Vec<WeatherCondition>,
    pub wind_shears: Vec<WindShear>,
    /// Sea groups.
    ///
    /// JSON representation is flattened once.
    #[serde(flatten)]
    pub sea: Sea,
    pub trend_changes: Vec<TrendChange>,
    pub report: String,
}

/// Decodes a METAR report into a [Metar] struct.
///
/// # Arguments
///
/// * `report` - METAR report to decode.
/// * `anchor_time` - Specifies a datetime that is ideally close to that one when the report was actually published.
///                   If given, the decoded METAR day and time will be converted to a full datetime. See also [MetarTime::to_date_time()].
pub fn decode_metar(report: &str, anchor_time: Option<NaiveDateTime>) -> Result<Metar> {
    let mut sanitized = report.to_uppercase().trim().replace('\x00', "");
    sanitized = WHITESPACE_REPLACE_RE.replace_all(&sanitized, *WHITESPACE_REPLACE_OUT).to_string();
    let report = END_REPLACE_RE.replace_all(&sanitized, *END_REPLACE_OUT).to_string();

    let mut section = Section::Main;

    let mut metar = Metar::default();
    metar.report = report.trim().to_string();

    let mut processing_trend_change = false;
    let mut trend_change = TrendChange::default();

    let mut unparsed_groups = Vec::new();

    // Handlers return mostly `Option<(some struct, end index)>` which gives us:
    // - None => the handler didn't parse the group which often leads to trying an another handler
    // - Some(some struct, end index) => the handler parsed the group (to some struct) and also returned an index of the group end
    //                                   which enables to further slice the report for other handlers to work with
    //
    // In certain cases, some struct may be empty (determined by `.is_empty()`) because all of its fields are missing.
    // For example, this typically happens when clouds are unknown (//////) and such empty struct will be skipped.

    let mut idx = 0;

    while idx < report.len() {
        let sub_report = &report[idx..];

        if let Some((sec, relative_end)) = handle_section(sub_report) {
            section = sec;
            idx += relative_end;

            if processing_trend_change {
                metar.trend_changes.push(trend_change.clone());
                processing_trend_change = false;
                trend_change = TrendChange::default();
            }

            if let Section::Trend(trend) = section {
                processing_trend_change = true;
                trend_change.indicator = trend;
            }

            continue;
        }

        match section {
            Section::Main => {
                if metar.header.is_empty() {
                    if let Some((header, relative_end)) = handle_header(sub_report, anchor_time) {
                        metar.header = header;
                        idx += relative_end;
                        continue;
                    }
                }

                if metar.wind.is_empty() {
                    if let Some((wind, relative_end)) = handle_wind(sub_report) {
                        metar.wind = wind;
                        idx += relative_end;
                        continue;
                    }
                }

                if metar.visibility.is_empty() {
                    if let Some((visibility, is_cavok, relative_end)) = handle_visibility(sub_report) {
                        metar.visibility = visibility;

                        if is_cavok {
                            let cloud_layer = CloudLayer { cover: Some(CloudCover::CeilingOk) , height: None, cloud_type: None };
                            metar.clouds.push(cloud_layer);
                        }

                        idx += relative_end;
                        continue;
                    }
                }

                if let Some((weather_condition, relative_end)) = handle_present_weather(sub_report) {
                    metar.present_weather.push(weather_condition);
                    idx += relative_end;
                    continue;
                }

                if let Some((runway_visual_range, relative_end)) = handle_runway_visual_range(sub_report) {
                    metar.runway_visual_ranges.push(runway_visual_range);
                    idx += relative_end;
                    continue;
                }

                if let Some((cloud_layer, relative_end)) = handle_cloud_layer(sub_report) {
                    if !cloud_layer.is_empty() {
                        metar.clouds.push(cloud_layer);
                    }

                    idx += relative_end;
                    continue;
                }

                if metar.temperature.is_empty() {
                    if let Some((temperature, relative_end)) = handle_temperature(sub_report) {
                        if !temperature.is_empty() {
                            metar.temperature = temperature;
                        }

                        idx += relative_end;
                        continue;
                    }
                }

                if metar.pressure.is_empty() {
                    if let Some((pressure, relative_end)) = handle_pressure(sub_report) {
                        metar.pressure = pressure;
                        idx += relative_end;
                        continue;
                    }
                }

                if let Some((weather_condition, relative_end)) = handle_recent_weather(sub_report) {
                    metar.recent_weather.push(weather_condition);
                    idx += relative_end;
                    continue;
                }

                if let Some((wind_shear, relative_end)) = handle_wind_shear(sub_report) {
                    metar.wind_shears.push(wind_shear);
                    idx += relative_end;
                    continue;
                }

                if metar.sea.is_empty() {
                    if let Some((sea, relative_end)) = handle_sea(sub_report) {
                        if !sea.is_empty() {
                            metar.sea = sea;
                        }

                        idx += relative_end;
                        continue;
                    }
                }

                // Colour state, won't store. For more info check:
                // <https://en.wikipedia.org/wiki/Colour_state>
                if let Some(relative_end) = handle_color(sub_report) {
                    idx += relative_end;
                    continue;
                }

                // Rainfall in last 10min / since 0900 local time, won't store. For more info check:
                // <http://www.bom.gov.au/aviation/Aerodrome/metar-speci.pdf>
                if let Some(relative_end) = handle_rainfall(sub_report) {
                    idx += relative_end;
                    continue;
                }

                // Runway state (should be part of SNOWTAM), won't store. For more info check:
                // <https://www.icao.int/WACAF/Documents/Meetings/2021/GRF/2.%20Provisions%20on%20GRF.pdf>
                if let Some(relative_end) = handle_runway_state(sub_report) {
                    idx += relative_end;
                    continue;
                }
            },
            Section::Trend(_) => {
                if let Some((trend_time, relative_end)) = handle_trend_time(sub_report, anchor_time) {
                    match trend_time.indicator {
                        TrendTimeIndicator::From => {
                            trend_change.from_time = trend_time.time;
                        },
                        TrendTimeIndicator::Until => {
                            trend_change.to_time = trend_time.time;
                        },
                        TrendTimeIndicator::At => {
                            trend_change.at_time = trend_time.time;
                        },
                    }

                    idx += relative_end;
                    continue;
                }

                if trend_change.wind.is_empty() {
                    if let Some((wind, relative_end)) = handle_wind(sub_report) {
                        trend_change.wind = wind;
                        idx += relative_end;
                        continue;
                    }
                }

                if trend_change.visibility.is_empty() {
                    if let Some((visibility, is_cavok, relative_end)) = handle_visibility(sub_report) {
                        trend_change.visibility = visibility;

                        if is_cavok {
                            let cloud_layer = CloudLayer { cover: Some(CloudCover::CeilingOk) , height: None, cloud_type: None };
                            trend_change.clouds.push(cloud_layer);
                        }

                        idx += relative_end;
                        continue;
                    }
                }

                if let Some((weather_condition, relative_end)) = handle_present_weather(sub_report) {
                    trend_change.weather.push(weather_condition);
                    idx += relative_end;
                    continue;
                }

                if let Some((cloud_layer, relative_end)) = handle_cloud_layer(sub_report) {
                    if !cloud_layer.is_empty() {
                        trend_change.clouds.push(cloud_layer);
                    }

                    idx += relative_end;
                    continue;
                }
            },
            Section::Remark => (), // TODO: https://github.com/meandair/rweather-decoder/issues/15
        }

        let relative_end = sub_report.find(' ').unwrap();

        let unparsed = &report[idx..idx + relative_end];
        if unparsed.chars().any(|c| c != '/') {
            unparsed_groups.push(unparsed);
        }

        idx += relative_end + 1;
    }

    if processing_trend_change {
        metar.trend_changes.push(trend_change);
    }

    if !unparsed_groups.is_empty() {
        log::debug!("Unparsed data: {}, report: {}", unparsed_groups.join(" "), report);
    }

    Ok(metar)
}
