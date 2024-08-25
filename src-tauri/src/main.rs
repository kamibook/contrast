#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use chrono::{DateTime, Utc};
use lazy_static::lazy_static;
use regex::Regex;
use rodio;
use simple_log::{info, LogConfigBuilder};
use std::fs::File;
use std::io::{BufReader, Error, ErrorKind, Read};
use std::path::Path;
use std::result::Result;
use toml::Value;

lazy_static! {
    static ref SN_REGEXES: Vec<Regex> = load_regex_rules("rules.toml").unwrap().0;
    static ref PAPER_REGEXES: Vec<Regex> = load_regex_rules("rules.toml").unwrap().1;
}

#[tauri::command]
fn sp_contrast(sn: &str, paper: &str) -> String {
    let now: DateTime<Utc> = Utc::now();
    let formatted_now = now.format("%Y-%m-%d %H:%M:%S").to_string();

    let (sn_match, paper_match) = extract_content(sn, paper, &SN_REGEXES, &PAPER_REGEXES);

    match (sn_match, paper_match) {
        (Some(sn_val), Some(paper_val)) => compare_and_play_audio(formatted_now, sn_val, paper_val),
        (None, _) | (_, None) => play_audio_and_return_message("audio/fail.wav", "有一个为 None"),
    }
}

fn main() {
    sp_log().unwrap();
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![sp_contrast])
        .run(tauri::generate_context!())
        .expect("错误： 启动失败！");
}

fn compare_and_play_audio(formatted_now: String, sn_val: String, paper_val: String) -> String {
    if sn_val == paper_val {
        let success_msg = format!("{}: {} {}  对比成功！", formatted_now, sn_val, paper_val);
        info!("{} {}  对比成功！", sn_val, paper_val);
        play_audio_and_return_message("audio/pass.wav", &success_msg)
    } else {
        let fail_msg = format!("{}: {} {}  对比失败！", formatted_now, sn_val, paper_val);
        info!("{} {}  对比失败！", sn_val, paper_val);
        play_audio_and_return_message("audio/fail.wav", &fail_msg)
    }
}

fn play_audio_and_return_message(audio_file: &str, message: &str) -> String {
    if let Err(e) = play_audio(audio_file) {
        format!("Error: {}", e)
    } else {
        message.to_string()
    }
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

fn play_audio(file_path: &str) -> Result<(), Error> {
    if !Path::new(file_path).exists() {
        return Err(Error::new(
            ErrorKind::NotFound,
            format!("音频文件 {} 不存在。", file_path),
        ));
    }

    let (_stream, handle) = rodio::OutputStream::try_default().unwrap();
    let sink = rodio::Sink::try_new(&handle).unwrap();

    let file = std::fs::File::open(file_path).unwrap();
    sink.append(rodio::Decoder::new(BufReader::new(file)).unwrap());
    sink.sleep_until_end();

    Ok(())
}

fn load_regex_rules(
    rules_path: &str,
) -> Result<(Vec<Regex>, Vec<Regex>), Box<dyn std::error::Error>> {
    let mut file = File::open(rules_path)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;

    let toml_value: Value = toml::from_str(&contents)?;
    let sn_rules_vec = extract_rules(&toml_value, "sn_rules")?;
    let paper_rules_vec = extract_rules(&toml_value, "paper_rules")?;

    Ok((sn_rules_vec, paper_rules_vec))
}

fn extract_rules(
    toml_value: &Value,
    rule_type: &str,
) -> Result<Vec<Regex>, Box<dyn std::error::Error>> {
    let rules_table = toml_value.get(rule_type).unwrap().as_table().unwrap();
    let rules_vec = rules_table
        .values()
        .map(|v| v.as_str().unwrap())
        .collect::<Vec<&str>>();

    let regexes = rules_vec
        .iter()
        .map(|regex_str| {
            Regex::new(regex_str).map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
        })
        .collect::<Result<Vec<Regex>, Box<dyn std::error::Error>>>()?;

    Ok(regexes)
}

fn trim_whitespace(s: &str) -> String {
    s.trim().to_string()
}

fn extract_content(
    sn: &str,
    paper: &str,
    sn_regexes: &[Regex],
    paper_regexes: &[Regex],
) -> (Option<String>, Option<String>) {
    let sn_match = process_match(sn, sn_regexes);
    let paper_match = process_match(paper, paper_regexes);

    (sn_match, paper_match)
}

fn process_match(s: &str, regexes: &[Regex]) -> Option<String> {
    if regexes.is_empty() {
        Some(trim_whitespace(s))
    } else {
        regexes.iter().find_map(|regex| {
            regex
                .captures(s)
                .map(|caps| {
                    if let Some(m) = caps.get(1) {
                        trim_whitespace(m.as_str())
                    } else {
                        trim_whitespace(&caps[0])
                    }
                })
                .or_else(|| Some(trim_whitespace(s)))
        })
    }
}
