//! Decode METAR reports stored in various file formats and save them into a JSON file.

use std::{
    collections::HashSet,
    fs::File,
    io::{BufRead, BufReader, BufWriter, Write},
    path::{Path, PathBuf},
    str::FromStr,
};

use anyhow::{anyhow, Error, Result};
use chrono::{NaiveDateTime, ParseError};
use glob::glob;
use log::{info, warn};
use structopt::StructOpt;
use encoding_rs::WINDOWS_1252;
use encoding_rs_io::DecodeReaderBytesBuilder;

use rweather_decoder::metar;

/// METAR file formats.
enum MetarFileFormat {
    /// NOAA METAR cycle format as used at
    /// <https://tgftp.nws.noaa.gov/data/observations/metar/cycles/>.
    NoaaMetarCycles,
    /// Plain TXT format where each row represents one METAR report.
    Plain,
}

impl FromStr for MetarFileFormat {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "noaa-metar-cycles" => Ok(MetarFileFormat::NoaaMetarCycles),
            "plain" => Ok(MetarFileFormat::Plain),
            _ => Err(anyhow!("Invalid METAR file format, given {}", s))
        }
    }
}

/// Decode METAR reports in a file with NOAA METAR cycle format.
fn decode_noaa_metar_cycles_file(path: &Path) -> Result<Vec<metar::Metar>> {
    let file = File::open(path)?;
    let enc_reader = DecodeReaderBytesBuilder::new()
        .encoding(Some(WINDOWS_1252))
        .build(file);
    let buf_reader = BufReader::new(enc_reader);

    let mut data = Vec::new();
    let mut is_header = true; // some files start with garbage rows

    for row in buf_reader.lines() {
        let row = row?.trim().to_string();

        if is_header {
            let time_as_number = row.replace(['/', ' ', ':'], "");
            if time_as_number.parse::<usize>().is_ok() {
                data.push(row);
                is_header = false;
            }
        } else if !row.is_empty() {
            data.push(row);
        }
    }

    let mut all_metar_data = Vec::new();

    for (time_str, report) in data.iter().step_by(2).zip(data.iter().skip(1).step_by(2)) {
        let obs_time = NaiveDateTime::parse_from_str(time_str, "%Y/%m/%d %H:%M")?;

        match metar::decode_metar(report, Some(&obs_time)) {
            Ok(metar_data) => all_metar_data.push(metar_data),
            Err(e) => warn!("{}", e),
        }
    }

    Ok(all_metar_data)
}

/// Decode METAR reports in a file with plain format.
fn decode_plain_file(path: &Path, anchor_time: Option<&NaiveDateTime>) -> Result<Vec<metar::Metar>> {
    let file = File::open(path)?;
    let buf_reader = BufReader::new(file);

    let mut all_metar_data = Vec::new();

    for row in buf_reader.lines() {
        let row = row?.trim().to_string();

        if !row.is_empty() {
            match metar::decode_metar(&row, anchor_time) {
                Ok(metar_data) => all_metar_data.push(metar_data),
                Err(e) => warn!("{}", e),
            }
        }
    }

    Ok(all_metar_data)
}

fn naive_date_time_from_yyyy_mm_dd_str(s: &str) -> Result<NaiveDateTime, ParseError> {
    NaiveDateTime::parse_from_str(s, "%Y-%m-%d")
}

#[derive(StructOpt)]
/// CLI decoder of METAR reports
struct Cli {
    /// Quiet
    #[structopt(short, long)]
    quiet: bool,
    /// METAR file format (noaa-metar-cycles, plain)
    #[structopt(short, long, default_value = "noaa-metar-cycles")]
    file_format: MetarFileFormat,
    /// Enable pretty-printing of output JSON file
    #[structopt(short, long)]
    pretty_print: bool,
    /// Anchor time (YYYY-MM-DD) for the plain file format.
    /// Specifies a day close to the one when the reports were collected.
    /// If given, the individual METAR day will be matched against it
    /// to create a proper datetime representation.
    #[structopt(short, long, parse(try_from_str = naive_date_time_from_yyyy_mm_dd_str))]
    anchor_time: Option<NaiveDateTime>,
    /// Input files (glob patterns separated by space)
    #[structopt(required = true)]
    input_globs: Vec<String>,
    /// Output JSON file. Same input reports will be deduplicated.
    output: PathBuf,
}

fn main() -> Result<()> {
    let args = Cli::from_args();

    if !&args.quiet {
        env_logger::init();
    }

    info!("Reading input glob patterns");

    let mut input_paths = HashSet::new();

    for glob_pattern in args.input_globs.iter() {
        for input_path in glob(glob_pattern)? {
            input_paths.insert(input_path?);
        }
    }

    info!("Found {} file(s)", input_paths.len());

    let mut unique_reports = HashSet::new();
    let mut all_metars = Vec::new();

    for input_path in input_paths.iter() {
        let metars = match args.file_format {
            MetarFileFormat::NoaaMetarCycles => decode_noaa_metar_cycles_file(input_path)?,
            MetarFileFormat::Plain => decode_plain_file(input_path, args.anchor_time.as_ref())?,
        };

        for metar in metars.into_iter() {
            if unique_reports.contains(&metar.report) {
                continue;
            } else {
                unique_reports.insert(metar.report.clone());
                all_metars.push(metar);
            }
        }
    }

    info!("Saving to file {:?}", &args.output);

    let file = File::create(&args.output)?;
    let mut writer = BufWriter::new(file);

    if args.pretty_print {
        // pretty-printing is ~50% slower
        serde_json::to_writer_pretty(&mut writer, &all_metars)?;
    } else {
        serde_json::to_writer(&mut writer, &all_metars)?;
    }

    writer.flush()?;

    Ok(())
}
