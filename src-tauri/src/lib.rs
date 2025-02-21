/*
MIT License

Copyright (c) 2025 Moriya Harumi

Permission is hereby granted, free of charge, to any person obtaining
a copy of this software and associated documentation files (the
"Software"), to deal in the Software without restriction, including
without limitation the rights to use, copy, modify, merge, publish,
distribute, sublicense, and/or sell copies of the Software, and to
permit persons to whom the Software is furnished to do so, subject to
the following conditions:

The above copyright notice and this permission notice shall be
included in all copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF
MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND
NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE
LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION
WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

*/

use anyhow::Result;
use image::codecs::avif::AvifEncoder;
use image::ImageEncoder;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{BufReader, BufWriter, Read};
use std::path::PathBuf;
use tauri::Error;
use webp;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            convert,
            get_files_binary,
            save_files,
            remove_temp_dir,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[derive(Debug, Serialize, Deserialize)]
struct FileInfo {
    file_name: String,
    file_name_with_extension: String,
    mime_type: String,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
enum ExtensionType {
    Webp,
    Avif,
}

impl ExtensionType {
    fn get_extension_str(&self) -> &'static str {
        match self {
            ExtensionType::Webp => "webp",
            ExtensionType::Avif => "avif",
        }
    }
}

#[tauri::command]
fn convert(
    files_binary: Vec<Vec<u8>>,
    file_infos: Vec<FileInfo>,
    extension_type: ExtensionType,
    quality: u8,
) -> Result<(Vec<PathBuf>, String), Error> {
    // 一時ディレクトリを取得
    let temp_dir = std::env::temp_dir();
    // 一意なディレクトリ名を生成
    let unique_dir_name = uuid::Uuid::new_v4().to_string();
    // 一時ファイルを保存する一時ディレクトリを作成
    let output_dir = temp_dir.join(format!("tavif_{}", unique_dir_name));
    std::fs::create_dir_all(&output_dir)?;

    let output_paths = files_binary
        .par_iter()
        .zip(file_infos)
        .filter_map(|(file_binary, file_info)| {
            let output_path = output_dir.join(format!(
                "{}.{}",
                file_info.file_name,
                extension_type.get_extension_str()
            ));
            match extension_type {
                ExtensionType::Webp => {
                    encode_to_webp(file_binary, &output_path, quality).ok()?;
                }
                ExtensionType::Avif => {
                    encode_to_avif(file_binary, &output_path, quality).ok()?;
                }
            };
            Some(output_path)
        })
        .collect::<Vec<_>>();

    Ok((output_paths, output_dir.to_str().unwrap_or("").to_string()))
}

fn encode_to_avif(img_binary: &[u8], output_path: &std::path::Path, quality: u8) -> Result<()> {
    let img = image::load_from_memory(img_binary)?;
    let img = img.to_rgb8();
    let file = File::create(output_path)?;

    let writer = BufWriter::new(file);
    let encoder = AvifEncoder::new_with_speed_quality(writer, 5, quality);
    encoder.write_image(
        img.as_raw(),
        img.width(),
        img.height(),
        image::ColorType::Rgb8,
    )?;
    Ok(())
}

fn encode_to_webp(img_binary: &[u8], output_path: &std::path::Path, quality: u8) -> Result<()> {
    // 引数のqualityはu8型で統一しているため、ここでf32型に変換する
    let quality = quality as f32;

    let img = image::load_from_memory(&img_binary)?;
    let img = img.to_rgba8();
    let (width, height) = img.dimensions();

    let encoded = webp::Encoder::new(&*img, webp::PixelLayout::Rgba, width, height)
        .encode(quality)
        .to_vec();

    // エンコードされたデータをファイルに書き込む
    std::fs::write(output_path, encoded)?;
    Ok(())
}

#[tauri::command]
fn get_files_binary(file_paths: Vec<String>) -> Result<Vec<Vec<u8>>, Error> {
    let mut files_binary = Vec::new();
    for file_path in file_paths {
        let file = File::open(file_path)?;
        let mut reader = BufReader::new(file);
        let mut buffer = Vec::new();
        reader.read_to_end(&mut buffer)?;
        files_binary.push(buffer);
    }
    Ok(files_binary)
}

#[tauri::command]
fn save_files(file_paths: Vec<String>, output_dir: String) -> Result<(), Error> {
    for file_path in file_paths {
        let file = File::open(&file_path)?;
        let file_name = file_path.split("\\").last().unwrap().to_string();
        let output_path = output_dir.clone() + "/" + &file_name;
        let mut reader = BufReader::new(file);
        let mut buffer = Vec::new();
        reader.read_to_end(&mut buffer)?;
        std::fs::write(output_path, buffer)?;
    }
    Ok(())
}

#[tauri::command]
fn remove_temp_dir(output_temp_dir: String) -> Result<(), Error> {
    // ディレクトリが存在する場合は削除
    if std::fs::remove_dir_all(&output_temp_dir).is_ok() {
        Ok(())
    } else {
        return Err(Error::from(anyhow::anyhow!(
            "Failed to remove temp directory"
        )));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;

    #[test]
    fn test_convert() {
        // テスト用の画像データを準備
        let image_data = include_bytes!("../../public/null.png").to_vec(); // 実際の画像ファイルを指定
        let files_binary = vec![image_data]; // ダミーデータを実際の画像データに変更
        let file_infos = vec![FileInfo {
            file_name: "test".to_string(),
            file_name_with_extension: "test.webp".to_string(),
            mime_type: "image/webp".to_string(),
        }];
        let extension_type = ExtensionType::Webp;
        let quality = 75;

        // convert関数を呼び出す
        let result = convert(files_binary, file_infos, extension_type, quality);

        // 結果を検証
        assert!(result.is_ok());
        let (output_paths, output_dir) = result.unwrap();
        assert_eq!(output_paths.len(), 1);
        assert!(Path::new(&output_paths[0]).exists()); // 出力ファイルが存在することを確認
        assert!(Path::new(&output_dir).exists()); // 出力ディレクトリが存在することを確認
    }

    #[test]
    fn test_get_files_binary() {
        // テスト用のファイルを作成
        let test_file_path = "test_file.txt";
        fs::write(test_file_path, b"Hello, world!").unwrap();

        // get_files_binary関数を呼び出す
        let result = get_files_binary(vec![test_file_path.to_string()]);

        // 結果を検証
        assert!(result.is_ok());
        let files_binary = result.unwrap();
        assert_eq!(files_binary.len(), 1);
        assert_eq!(files_binary[0], b"Hello, world!");

        // テスト用のファイルを削除
        fs::remove_file(test_file_path).unwrap();
    }

    #[test]
    fn test_save_files() {
        // テスト用のデータを準備
        let test_file_path = "test_save_file.txt";
        fs::write(test_file_path, b"Test data").unwrap();
        let output_dir = "."; // 現在のディレクトリに保存

        // save_files関数を呼び出す
        let result = save_files(vec![test_file_path.to_string()], output_dir.to_string());

        // 結果を検証
        assert!(result.is_ok());
        assert!(Path::new(test_file_path).exists()); // 元のファイルが存在することを確認

        // テスト用のファイルを削除
        fs::remove_file(test_file_path).unwrap();
    }

    #[test]
    fn test_remove_temp_dir() {
        let output_dir = "test_remove_temp_dir";
        std::fs::create_dir_all(output_dir).unwrap();
        let result = remove_temp_dir(output_dir.to_string());
        assert!(result.is_ok());
    }
}
