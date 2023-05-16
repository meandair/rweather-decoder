//! METAR decoding module.

use std::{ops::{Div, Mul}, str::FromStr};

use anyhow::{anyhow, Error, Result};
use chrono::{NaiveDateTime, NaiveTime, Datelike, Duration};
use chronoutil::RelativeDuration;
use lazy_static::lazy_static;
use log::debug;
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
        (?P<minute>\d\d)Z?
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
        ^(?P<prevailing>[MP]?(\d+\s)?\d/\d|[MP]?\d{1,4}|////|CAVOK)
        (NDV)?
        \s?
        (?P<units>SM)?
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
        (?P<vicinity>VC)?
        (?P<descriptor>MI|BC|PR|DR|BL|SH|TS|FZ)?
        (?P<phenomena>(DZ|RA|SN|SG|PL|GR|GS|UP|BR|FG|FU|VA|DU|SA|HZ|PO|SQ|FC|SS|DS|IC|PY)+)?
        (?P<end>\s)
    ").unwrap();

    static ref CLOUD_RE: Regex = Regex::new(r"(?x)
        ^(?P<cover>CLR|SKC|NSC|NCD|FEW|SCT|BKN|OVC|VV|///)
        (?P<height>\d\d\d|///)?
        (?P<cloud>AC|ACC|ACSL|AS|CB|CBMAM|CC|CCSL|CI|CS|CU|NS|SC|SCSL|ST|TCU|///)?
        (?P<end>\s)
    ").unwrap();

    static ref TEMPERATURE_RE: Regex = Regex::new(r"(?x)
        ^(?P<temperature>M?\d\d|//)
        /
        (?P<dew_point>M?\d\d|//)?
        (?P<end>\s)
    ").unwrap();

    static ref PRESSURE_RE: Regex = Regex::new(r"(?x)
        ^(?P<units>A|Q)
        (?P<pressure>\d\d\d\d|////)
        (?P<end>\s)
    ").unwrap();

    static ref RECENT_WEATHER_RE: Regex = Regex::new(r"(?x)
        ^RE(?P<intensity>[-\+])?
        (?P<vicinity>VC)?
        (?P<descriptor>MI|BC|PR|DR|BL|SH|TS|FZ)?
        (?P<phenomena>(DZ|RA|SN|SG|PL|GR|GS|UP|BR|FG|FU|VA|DU|SA|HZ|PO|SQ|FC|SS|DS|IC|PY)+)?
        (?P<end>\s)
    ").unwrap();

    static ref COLOR_RE: Regex = Regex::new(r"(?x)
        ^(BLACK|BLU\+?|GRN|WHT|RED|AMB|YLO)+
        (?P<end>\s)
    ").unwrap();
}

#[non_exhaustive]
#[derive(Debug, PartialEq)]
enum Trend {
    NoSignificantChange,
    Temporary,
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
#[derive(Debug, PartialEq)]
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

#[non_exhaustive]
#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "value_type", content = "value", rename_all = "snake_case")]
enum MetarTime {
    DateTime(UtcDateTime),
    DayTime(UtcDayTime),
    Time(UtcTime),
}

impl MetarTime {
    fn to_date_time(&self, anchor_time: &NaiveDateTime) -> MetarTime {
        match self {
            MetarTime::DateTime(utc_dt) => MetarTime::DateTime(*utc_dt),
            MetarTime::DayTime(utc_d_t) => {
                let first_guess_opt = anchor_time.date().with_day(utc_d_t.0).map(|nd| nd.and_time(utc_d_t.1));
                let second_guess_opt = (*anchor_time + RelativeDuration::months(-1)).date().with_day(utc_d_t.0).map(|nd| nd.and_time(utc_d_t.1));
                let third_guess_opt = (*anchor_time + RelativeDuration::months(1)).date().with_day(utc_d_t.0).map(|nd| nd.and_time(utc_d_t.1));

                let mut final_guess_opt = None;
                let mut final_delta = i64::MAX;

                for guess_opt in [first_guess_opt, second_guess_opt, third_guess_opt] {
                    if let Some(guess) = guess_opt {
                        let delta = guess.signed_duration_since(*anchor_time).num_seconds().abs();
                        if delta < final_delta {
                            final_guess_opt = guess_opt;
                            final_delta = delta;
                        }
                    }
                }

                match final_guess_opt {
                    Some(final_guess) => MetarTime::DateTime(UtcDateTime(final_guess)),
                    // TODO: Make date guessing more robust and correctly handle the error.
                    None => panic!("{}", format!("Date guessing failed, given time {:?} and anchor time {}", self, anchor_time))
                }
            },
            MetarTime::Time(utc_t) => {
                let first_guess = anchor_time.date().and_time(utc_t.0);
                let second_guess = first_guess + Duration::days(-1);
                let third_guess = first_guess + Duration::days(1);

                let mut final_guess = first_guess;
                let mut final_delta = final_guess.signed_duration_since(*anchor_time).num_seconds().abs();

                for guess in [second_guess, third_guess] {
                    let delta = guess.signed_duration_since(*anchor_time).num_seconds().abs();
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

#[non_exhaustive]
#[derive(Debug, PartialEq, Default, Serialize, Deserialize)]
struct Header {
    station_id: Option<String>,
    observation_time: Option<MetarTime>,
    is_corrected: Option<bool>,
    is_automated: Option<bool>,
}

fn handle_header(text: &str, anchor_time: Option<&NaiveDateTime>) -> Option<(Header, usize)> {
    HEADER_RE.captures(text)
        .map(|capture| {
            let station_id = Some(capture["station_id"].to_string());

            let day = capture["day"].parse().unwrap();
            let hour = capture["hour"].parse().unwrap();
            let minute = capture["minute"].parse().unwrap();

            let mut time = Some(MetarTime::DayTime(UtcDayTime(day, NaiveTime::from_hms_opt(hour, minute, 0).unwrap())));

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

#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
enum Unit {
    #[serde(rename = "degT")]
    DegreeTrue,
    #[serde(rename = "kt")]
    Knot,
    #[serde(rename = "m/s")]
    MetrePerSecond,
    #[serde(rename = "m")]
    Metre,
    #[serde(rename = "mi")]
    StatuteMile,
    #[serde(rename = "ft")]
    Foot,
    #[serde(rename = "degC")]
    DegreeCelsius,
    #[serde(rename = "hPa")]
    HectoPascal,
    #[serde(rename = "inHg")]
    InchOfMercury,
}

impl FromStr for Unit {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "KT" => Ok(Unit::Knot),
            "MPS" => Ok(Unit::MetrePerSecond),
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

#[non_exhaustive]
#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "value_type", content = "value", rename_all = "snake_case")]
enum ValueInRange {
    Above(f32),
    Below(f32),
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

#[non_exhaustive]
#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "value_type", content = "value", rename_all = "snake_case")]
enum Value {
    Variable,
    Above(f32),
    Below(f32),
    Range(ValueInRange, ValueInRange),
    Exact(f32),
    Unlimited,
    Indefinite,
}

impl FromStr for Value {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s == "VRB" {
            Ok(Value::Variable)
        } else if let Some(stripped) = s.strip_prefix('P') {
            let value = parse_value(stripped).unwrap();
            Ok(Value::Above(value))
        } else if let Some(stripped) = s.strip_prefix('M') {
            let value = parse_value(stripped).unwrap();
            Ok(Value::Below(value))
        } else if s.contains('V') {
            let mut split = s.split('V');
            let value1 = ValueInRange::from_str(split.next().unwrap()).unwrap();
            let value2 = ValueInRange::from_str(split.next().unwrap()).unwrap();
            Ok(Value::Range(value1, value2))
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
            Value::Unlimited => Value::Unlimited,
            Value::Indefinite => Value::Indefinite,
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
            Value::Unlimited => Value::Unlimited,
            Value::Indefinite => Value::Indefinite,
        }
    }
}

#[non_exhaustive]
#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
struct Quantity {
    #[serde(flatten)]
    value: Value,
    units: Unit,
}

impl Quantity {
    fn new(value: Value, units: Unit) -> Quantity {
        Quantity { value, units }
    }

    fn new_opt(value: Option<Value>, units: Unit) -> Option<Quantity> {
        value.map(|v| Quantity { value: v, units })
    }
}

#[non_exhaustive]
#[derive(Debug, PartialEq, Default, Serialize, Deserialize)]
struct Wind {
    wind_from_direction: Option<Quantity>,
    wind_from_direction_range: Option<Quantity>,
    wind_speed: Option<Quantity>,
    wind_gust: Option<Quantity>,
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

#[non_exhaustive]
#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum DirectionOctant {
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

#[non_exhaustive]
#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct DirectionalVisibility {
    visibility: Quantity,
    direction: DirectionOctant,
}

#[non_exhaustive]
#[derive(Debug, PartialEq, Default, Serialize, Deserialize)]
struct Visibility {
    prevailing_visibility: Option<Quantity>,
    minimum_visibility: Option<Quantity>,
    directional_visibilites: Vec<DirectionalVisibility>,
}

fn handle_visibility(text: &str) -> Option<(Visibility, bool, usize)> {
    VISIBILITY_RE.captures(text)
        .map(|capture| {
            let mut is_cavok = false;

            let mut prevailing_visibility_value = match &capture["prevailing"] {
                "////" => None,
                "CAVOK" => {
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

#[non_exhaustive]
#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum RunwayVisualRangeTrend {
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

#[non_exhaustive]
#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct RunwayVisualRange {
    runway: String,
    visual_range: Quantity,
    trend: Option<RunwayVisualRangeTrend>,
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

#[non_exhaustive]
#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum WeatherIntensity {
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

#[non_exhaustive]
#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum WeatherDescriptor {
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

#[non_exhaustive]
#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum WeatherPhenomena {
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
            _ => Err(anyhow!("Invalid weather phenomena, given {}", s))
        }
    }
}

#[non_exhaustive]
#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct WeatherCondition {
    intensity: WeatherIntensity,
    is_in_vicinity: bool,
    descriptor: Option<WeatherDescriptor>,
    phenomena: Vec<WeatherPhenomena>,
}

fn handle_weather(weather_re: &Regex, text: &str) -> Option<(WeatherCondition, usize)> {
    weather_re.captures(text)
        .map(|capture| {
            let intensity = capture.name("intensity")
                .map(|c| WeatherIntensity::from_str(c.as_str()).unwrap())
                .unwrap_or(WeatherIntensity::Moderate);

            let is_in_vicinity = capture.name("vicinity").is_some();

            let descriptor = capture.name("descriptor")
                .map(|c| WeatherDescriptor::from_str(c.as_str()).unwrap());

            let phenomena = capture.name("phenomena")
                .map(|c| c.as_str().chars().collect::<Vec<_>>()
                    .chunks(2)
                    .map(|chunk| WeatherPhenomena::from_str(&chunk.iter().collect::<String>()).unwrap())
                    .collect())
                .unwrap_or_default();

            let end = capture.name("end").unwrap().end();

            let weather = WeatherCondition { intensity, is_in_vicinity, descriptor, phenomena };

            (weather, end)
        })
}

fn handle_present_weather(text: &str) -> Option<(WeatherCondition, usize)> {
    handle_weather(&PRESENT_WEATHER_RE, text)
}

fn handle_recent_weather(text: &str) -> Option<(WeatherCondition, usize)> {
    handle_weather(&RECENT_WEATHER_RE, text)
}

#[non_exhaustive]
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum CloudCover {
    Clear,
    SkyClear,
    NilSignificantCloud,
    NoCloudDetected,
    Few,
    Scattered,
    Broken,
    Overcast,
    VerticalVisibility,
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

#[non_exhaustive]
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum CloudType {
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
            "TCU" => Ok(CloudType::ToweringCumulus),
            _ => Err(anyhow!("Invalid cloud type, given {s}"))
        }
    }
}

#[non_exhaustive]
#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct CloudLayer {
    cover: Option<CloudCover>,
    /// Height above ground level (AGL).
    height: Option<Quantity>,
    cloud_type: Option<CloudType>,
}

impl CloudLayer {
    fn has_some(&self) -> bool {
        self.cover.is_some() || self.height.is_some() || self.cloud_type.is_some()
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

fn calculate_ceiling(cloud_layers: &[CloudLayer]) -> Option<Quantity> {
    const UNDEFINED_CEILING: f32 = 999999.9;

    let mut min_ceiling = UNDEFINED_CEILING;
    let mut is_unlimited_opt = None;

    for cloud_layer in cloud_layers {
        match cloud_layer.cover {
            Some(CloudCover::Clear) | Some(CloudCover::SkyClear) | Some(CloudCover::NilSignificantCloud)
                    | Some(CloudCover::NoCloudDetected) | Some(CloudCover::Few) | Some(CloudCover::Scattered)
                    | Some(CloudCover::CeilingOk) => {
                if is_unlimited_opt.is_none() {
                    is_unlimited_opt = Some(true);
                }
            },
            Some(CloudCover::Broken) | Some(CloudCover::Overcast) | Some(CloudCover::VerticalVisibility) => {
                if let Some(height_quantity) = &cloud_layer.height {
                    let height = match height_quantity.value {
                        Value::Exact(h) => h,
                        _ => unreachable!(),
                    };

                    min_ceiling = min_ceiling.min(height);
                }

                is_unlimited_opt = Some(false);
            },
            None => (),
        }
    }

    match is_unlimited_opt {
        Some(is_unlimited) => match is_unlimited {
            true => Some(Quantity::new(Value::Unlimited, Unit::Foot)),
            false => if min_ceiling < 999999.0 {
                Some(Quantity::new(Value::Exact(min_ceiling), Unit::Foot))
            } else {
                Some(Quantity::new(Value::Indefinite, Unit::Foot))
            },
        }
        None => None
    }
}

#[non_exhaustive]
#[derive(Debug, PartialEq, Default, Serialize, Deserialize)]
struct Temperature {
    temperature: Option<Quantity>,
    dew_point: Option<Quantity>,
}

fn handle_temperature(text: &str) -> Option<(Temperature, usize)> {
    TEMPERATURE_RE.captures(text)
        .map(|capture| {
            let temperature_value = match &capture["temperature"] {
                "//" => None,
                s => Some(Value::from_str(&s.replace('M', "-")).unwrap()),
            };

            let dew_point_value = capture.name("dew_point").and_then(|c| match c.as_str() {
                "//" => None,
                s => Some(Value::from_str(&s.replace('M', "-")).unwrap()),
            });

            let temperature = Quantity::new_opt(temperature_value, Unit::DegreeCelsius);
            let dew_point = Quantity::new_opt(dew_point_value, Unit::DegreeCelsius);

            let end = capture.name("end").unwrap().end();

            let temperature = Temperature { temperature, dew_point };

            (temperature, end)
        })
}

#[non_exhaustive]
#[derive(Debug, PartialEq, Default, Serialize, Deserialize)]
struct Pressure {
    pressure: Option<Quantity>,
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

fn handle_color(text: &str) -> Option<usize> {
    COLOR_RE.captures(text)
        .map(|capture| {
            let end = capture.name("end").unwrap().end();

            end
        })
}

/// A decoded METAR report.
#[non_exhaustive]
#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Metar {
    #[serde(flatten)]
    header: Header,
    #[serde(flatten)]
    wind: Wind,
    #[serde(flatten)]
    visibility: Visibility,
    runway_visual_ranges: Vec<RunwayVisualRange>,
    present_weather: Vec<WeatherCondition>,
    clouds: Vec<CloudLayer>,
    ceiling: Option<Quantity>,
    #[serde(flatten)]
    temperature: Temperature,
    #[serde(flatten)]
    pressure: Pressure,
    recent_weather: Vec<WeatherCondition>,
    pub report: String,
}

/// Decodes METAR report into [Metar] struct.
///
/// The optional `anchor_time` specifies a day close the the one when the report was collected.
/// If given, the decoded METAR day and time will be matched against it to create [UtcDateTime]
/// struct which fully describes date and time.
pub fn decode_metar(report: &str, anchor_time: Option<&NaiveDateTime>) -> Result<Metar> {
    let mut sanitized = report.to_uppercase().trim().replace('\x00', "");
    sanitized = WHITESPACE_REPLACE_RE.replace_all(&sanitized, *WHITESPACE_REPLACE_OUT).to_string();
    let report = END_REPLACE_RE.replace_all(&sanitized, *END_REPLACE_OUT).to_string();

    let mut section = Section::Main;
    let mut header = None;
    let mut wind = None;
    let mut visibility = None;
    let mut runway_visual_ranges = Vec::new();
    let mut present_weather_conditions = Vec::new();
    let mut clouds = Vec::new();
    let mut temperature = None;
    let mut pressure = None;
    let mut recent_weather_conditions = Vec::new();

    let mut unparsed_groups = Vec::new();

    let mut idx = 0;

    while idx < report.len() {
        let sub_report = &report[idx..];

        if let Some((sec, relative_end)) = handle_section(sub_report) {
            section = sec;
            idx += relative_end;
            continue;
        }

        match section {
            Section::Main => {
                if header.is_none() {
                    if let Some((h, relative_end)) = handle_header(sub_report, anchor_time) {
                        header = Some(h);
                        idx += relative_end;
                        continue;
                    }
                }

                if wind.is_none() {
                    if let Some((w, relative_end)) = handle_wind(sub_report) {
                        wind = Some(w);
                        idx += relative_end;
                        continue;
                    }
                }

                if visibility.is_none() {
                    if let Some((vis, is_cavok, relative_end)) = handle_visibility(sub_report) {
                        visibility = Some(vis);

                        if is_cavok {
                            let cl = CloudLayer { cover: Some(CloudCover::CeilingOk) , height: None, cloud_type: None };
                            clouds.push(cl);
                        }

                        idx += relative_end;
                        continue;
                    }
                }

                if let Some((pw, relative_end)) = handle_present_weather(sub_report) {
                    present_weather_conditions.push(pw);
                    idx += relative_end;
                    continue;
                }

                if let Some((rvr, relative_end)) = handle_runway_visual_range(sub_report) {
                    runway_visual_ranges.push(rvr);
                    idx += relative_end;
                    continue;
                }

                if let Some((cl, relative_end)) = handle_cloud_layer(sub_report) {
                    if cl.has_some() {
                        clouds.push(cl);
                    }
                    idx += relative_end;
                    continue;
                }

                if temperature.is_none() {
                    if let Some((temp, relative_end)) = handle_temperature(sub_report) {
                        temperature = Some(temp);
                        idx += relative_end;
                        continue;
                    }
                }

                if let Some((p, relative_end)) = handle_pressure(sub_report) {
                    if pressure.is_none() {
                        pressure = Some(p);
                    }
                    idx += relative_end;
                    continue;
                }

                if let Some((rw, relative_end)) = handle_recent_weather(sub_report) {
                    recent_weather_conditions.push(rw);
                    idx += relative_end;
                    continue;
                }

                if let Some(relative_end) = handle_color(sub_report) {
                    idx += relative_end;
                    continue;
                }
            },
            Section::Trend(_) => (), // TODO: https://github.com/meandair/rweather-decoder/issues/14
            Section::Remark => (), // TODO: https://github.com/meandair/rweather-decoder/issues/15
        }

        let relative_end = sub_report.find(' ').unwrap();

        if section == Section::Main { // TODO: Push from all sections that are being decoded.
            let unparsed = &report[idx..idx + relative_end];
            if unparsed.chars().any(|c| c != '/') {
                unparsed_groups.push(unparsed);
            }
        }

        idx += relative_end + 1;
    }

    let report = report.trim().to_string();

    if !unparsed_groups.is_empty() {
        debug!("Unparsed data: {}, report: {}", unparsed_groups.join(" "), report);
    }

    let ceiling = calculate_ceiling(&clouds);

    Ok(Metar {
        header: header.unwrap_or_default(),
        wind: wind.unwrap_or_default(),
        visibility: visibility.unwrap_or_default(),
        present_weather: present_weather_conditions,
        runway_visual_ranges,
        clouds,
        ceiling,
        temperature: temperature.unwrap_or_default(),
        pressure: pressure.unwrap_or_default(),
        recent_weather: recent_weather_conditions,
        report,
    })
}
