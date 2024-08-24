#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use chrono::{DateTime, Utc};
use regex::Regex;
use rodio::{Decoder, OutputStream, Sink};
use simple_log::{info, LogConfigBuilder};
use std::fs::File;
use std::io::{BufReader, Error, ErrorKind, Read};
use std::path::Path;
use std::result::Result;
use toml::Value;


#[tauri::command]
fn sp_contrast(sn: &str, paper: &str) -> String {
    let now: DateTime<Utc> = Utc::now();
    let formatted_now = now.format("%Y-%m-%d %H:%M").to_string();

    let (sn_regexes, paper_regexes) = load_regex_rules("rules.toml").unwrap();
    let (sn_match, paper_match) = extract_content(sn, paper, &sn_regexes, &paper_regexes);

    match (sn_match, paper_match) {
        (Some(sn_val), Some(paper_val)) => {
            if sn_val == paper_val {
                let success_msg =
                    format!("{}: {} {}  对比成功！", formatted_now, sn_val, paper_val);
                info!("{} {}  对比成功！", sn_val, paper_val);
                if let Err(e) = play_audio("audio/pass.wav") {
                    format!("Error: {}", e)
                } else {
                    success_msg
                }
            } else {
                let fail_msg = format!("{}: {} {}  对比失败！", formatted_now, sn_val, paper_val);
                info!("{} {}  对比失败！", sn_val, paper_val);
                if let Err(e) = play_audio("audio/fail.wav") {
                    format!("Error: {}", e)
                } else {
                    fail_msg
                }
            }
        }
        (None, _) | (_, None) => {
            let none_msg = format!("{}: 有一个为 None", formatted_now);
            info!("有一个为 None");
            if let Err(e) = play_audio("audio/fail.wav") {
                format!("Error: {}", e)
            } else {
                none_msg
            }
        }
    }
}

fn main() {
    sp_log().unwrap();
    ensure_rules_toml_exists("rules.toml");
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![sp_contrast])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn play_audio(file_path: &str) -> Result<(), Error> {
    if !Path::new(file_path).exists() {
        return Err(Error::new(
            ErrorKind::NotFound,
            format!("音频文件 {} 不存在。", file_path),
        ));
    }

    let (_stream, handle) = OutputStream::try_default()
        .map_err(|e| Error::new(ErrorKind::Other, format!("创建输出流失败: {}", e)))?;

    let sink = Sink::try_new(&handle)
        .map_err(|e| Error::new(ErrorKind::Other, format!("创建音频接收器失败: {}", e)))?;

    let file = File::open(file_path)?;
    let source = Decoder::new(BufReader::new(file))
        .map_err(|e| Error::new(ErrorKind::Other, format!("解码音频文件失败: {}", e)))?;
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
    let sn_rules_vec = toml_value
        .get("sn_rules")
        .unwrap()
        .as_table()
        .unwrap()
        .values()
        .map(|v| v.as_str().unwrap())
        .collect::<Vec<&str>>();
    let paper_rules_vec = toml_value
        .get("paper_rules")
        .unwrap()
        .as_table()
        .unwrap()
        .values()
        .map(|v| v.as_str().unwrap())
        .collect::<Vec<&str>>();

    let sn_regexes = sn_rules_vec
        .iter()
        .map(|regex_str| {
            Regex::new(regex_str).map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
        })
        .collect::<Result<Vec<Regex>, Box<dyn std::error::Error>>>()?;

    let paper_regexes = paper_rules_vec
        .iter()
        .map(|regex_str| {
            Regex::new(regex_str).map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
        })
        .collect::<Result<Vec<Regex>, Box<dyn std::error::Error>>>()?;

    Ok((sn_regexes, paper_regexes))
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
    let sn_match = if sn_regexes.is_empty() {
        Some(trim_whitespace(sn))
    } else {
        sn_regexes.iter().find_map(|regex| {
            regex
                .captures(sn)
                .map(|caps| {
                    if let Some(m) = caps.get(1) {
                        trim_whitespace(m.as_str())
                    } else {
                        trim_whitespace(&caps[0])
                    }
                })
                .or_else(|| Some(trim_whitespace(sn)))
        })
    };
    let paper_match = if paper_regexes.is_empty() {
        Some(trim_whitespace(paper))
    } else {
        paper_regexes.iter().find_map(|regex| {
            regex
                .captures(sn)
                .map(|caps| {
                    if let Some(m) = caps.get(1) {
                        trim_whitespace(m.as_str())
                    } else {
                        trim_whitespace(&caps[0])
                    }
                })
                .or_else(|| Some(trim_whitespace(paper)))
        })
    };

    (sn_match, paper_match)
}

fn ensure_rules_toml_exists(rules_path: &str) {
    let default_content = r#"[sn_rules]
[paper_rules]"#;

    if !std::path::Path::new(rules_path).exists() {
        std::fs::write(rules_path, default_content).expect("Failed to create rules.toml");
    }
}
