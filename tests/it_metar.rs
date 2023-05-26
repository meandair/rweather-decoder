/// Integration tests for METAR.

use std::{path::PathBuf, process::Command, fs::File, io::BufReader};

use anyhow::Result;
use rweather_decoder::metar::Metar;
use tempfile::NamedTempFile;

fn run_decode_metar(input: &PathBuf, output: &PathBuf, file_format: &str) -> Result<()> {
    let binary_path = env!("CARGO_BIN_EXE_decode-metar");

    let status = Command::new(&binary_path)
        .args(&[
            input.as_os_str().to_str().unwrap(),
            output.as_os_str().to_str().unwrap(),
            "--quiet",
            "--file-format",
            file_format
        ])
        .status()?;
    assert!(status.success());

    Ok(())
}

fn it_metar_template(input: &str, given_output: &str, file_format: &str) -> Result<()> {
    let input_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests").join("data").join("metar").join(input);
    let given_output_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests").join("data").join("metar").join(given_output);

    let test_output = NamedTempFile::new_in(env!("CARGO_TARGET_TMPDIR"))?.into_temp_path();
    let test_output_path = test_output.to_path_buf();

    run_decode_metar(&input_path, &test_output_path, file_format)?;

    let file = File::open(&test_output_path)?;
    let buf_reader = BufReader::new(file);
    let test_data: Vec<Metar> = serde_json::from_reader(buf_reader)?;

    let file = File::open(&given_output_path)?;
    let buf_reader = BufReader::new(file);
    let given_data: Vec<Metar> = serde_json::from_reader(buf_reader)?;

    assert_eq!(test_data.len(), given_data.len());

    for (test_metar, given_metar) in test_data.iter().zip(given_data.iter()) {
        assert_eq!(test_metar, given_metar);
    }

    Ok(())
}

#[test]
fn it_metar_daytime() -> Result<()> {
    it_metar_template("it_daytime_input.txt", "it_daytime_output.json", "plain")
}

#[test]
fn it_metar_header() -> Result<()> {
    it_metar_template("it_header_input.txt", "it_header_output.json", "noaa-metar-cycles")
}

#[test]
fn it_metar_wind() -> Result<()> {
    it_metar_template("it_wind_input.txt", "it_wind_output.json", "noaa-metar-cycles")
}

#[test]
fn it_metar_visibility() -> Result<()> {
    it_metar_template("it_visibility_input.txt", "it_visibility_output.json", "noaa-metar-cycles")
}

#[test]
fn it_metar_rvr() -> Result<()> {
    it_metar_template("it_rvr_input.txt", "it_rvr_output.json", "noaa-metar-cycles")
}

#[test]
fn it_metar_temperature() -> Result<()> {
    it_metar_template("it_temperature_input.txt", "it_temperature_output.json", "noaa-metar-cycles")
}

#[test]
fn it_metar_pressure() -> Result<()> {
    it_metar_template("it_pressure_input.txt", "it_pressure_output.json", "noaa-metar-cycles")
}
