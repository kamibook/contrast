#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use chrono::{DateTime, Utc};
use regex::Regex;
use rodio::{Decoder, OutputStream, Sink};
use simple_log::{info, LogConfigBuilder};
use std::fs::File;
use std::io::{BufReader, Read};
use std::result::Result;
use toml::Value;

#[tauri::command]
fn sp_contrast(sn: &str, paper: &str) -> String {
    let now: DateTime<Utc> = Utc::now();
    let formatted_now = now.format("%Y-%m-%d %H:%M").to_string();
    let result_str: String;

    let (sn_regexes, paper_regexes) = load_regex_rules("rules.toml").unwrap();
    let (sn_match, paper_match) = extract_content(sn, paper, &sn_regexes, &paper_regexes);

    match (sn_match, paper_match) {
        (Some(sn_val), Some(paper_val)) => {
            if sn_val == paper_val {
                result_str = format!("{}: {} {}  对比成功！", formatted_now, sn_val, paper_val);
                info!("{} {}  对比成功！", sn_val, paper_val);
                let _ = play_audio("audio/pass.wav");
            } else {
                result_str = format!("{}: {} {}  对比失败！", formatted_now, sn_val, paper_val);
                info!("{} {}  对比失败！", sn_val, paper_val);
                let _ = play_audio("audio/fail.wav");
            }
        }
        (None, _) | (_, None) => {
            result_str = format!("{}: sn_match 或 paper_match 有一个为 None", formatted_now);
            info!("sn_match 或 paper_match 有一个为 None");
            let _ = play_audio("audio/fail.wav");
        }
    }
    result_str
}

fn main() {
    sp_log().unwrap();
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![sp_contrast])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn play_audio(file_path: &str) -> Result<(), std::io::Error> {
    let (_stream, handle) = OutputStream::try_default().unwrap();
    let sink = Sink::try_new(&handle).unwrap();
    let file = File::open(file_path).unwrap();
    let source = Decoder::new(BufReader::new(file)).unwrap();
    sink.append(source);
    sink.sleep_until_end();
    Ok(())
}

fn sp_log() -> Result<(), String> {
    let config = LogConfigBuilder::builder()
        .path("contrast.log")
        .size(1 * 100)
        .roll_count(2)
        .time_format("%Y-%m-%d %H:%M:%S")
        .level("info")
        .output_file()
        .output_console()
        .build();

    simple_log::new(config)?;
    Ok(())
}

fn load_regex_rules(
    rules_path: &str,
) -> Result<(Vec<Regex>, Vec<Regex>), Box<dyn std::error::Error>> {
    let mut file = File::open(rules_path)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;

    let toml_value: Value = toml::from_str(&contents)?;
    let sn_rules_value = toml_value.get("sn_rules").unwrap().as_table().unwrap();
    let paper_rules_value = toml_value.get("paper_rules").unwrap().as_table().unwrap();

    let sn_regexes: Result<Vec<Regex>, Box<dyn std::error::Error>> = sn_rules_value
        .iter()
        .map(|(_, value)| Regex::new(value.as_str().unwrap()))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.into());

    let paper_regexes: Result<Vec<Regex>, Box<dyn std::error::Error>> = paper_rules_value
        .iter()
        .map(|(_, value)| Regex::new(value.as_str().unwrap()))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.into());

    Ok((sn_regexes?, paper_regexes?))
}

fn extract_content(
    sn: &str,
    paper: &str,
    sn_regexes: &[Regex],
    paper_regexes: &[Regex],
) -> (Option<String>, Option<String>) {
    fn filter_alphanumeric(s: &str) -> String {
        s.chars().filter(|c| c.is_alphanumeric()).collect()
    }

    let sn_match = if sn_regexes.is_empty() {
        Some(sn.to_string())
    } else {
        sn_regexes.iter().find_map(|regex| {
            regex
                .captures(sn)
                .and_then(|caps| caps.get(1).map(|m| filter_alphanumeric(m.as_str())))
                .or_else(|| Some(filter_alphanumeric(sn)))
        })
    };

    let paper_match = if paper_regexes.is_empty() {
        Some(paper.to_string())
    } else {
        paper_regexes.iter().find_map(|regex| {
            regex
                .captures(paper)
                .and_then(|caps| caps.get(1).map(|m| filter_alphanumeric(m.as_str())))
                .or_else(|| Some(filter_alphanumeric(paper)))
        })
    };

    (sn_match, paper_match)
}
