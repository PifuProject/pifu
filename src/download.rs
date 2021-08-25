// Copyright (c) 2021 Xu Shaohua <shaohua@biofan.org>. All rights reserved.
// Use of this source is governed by General Public License that can be found
// in the LICENSE file.

use serde_derive::{Deserialize, Serialize};
use shell_rs::hashsum;
use std::fs::{create_dir_all, File};
use std::path::Path;

use crate::base::config::Arch;
use crate::config::get_binary_dir;
use crate::Error;

#[derive(Debug, Deserialize, Serialize)]
struct TaskList {
    appimagetool: Vec<Task>,
}

#[derive(Debug, Deserialize, Serialize)]
struct Task {
    arch: Arch,
    url: String,
    filename: String,
    sha256: String,
}

pub fn download() -> Result<(), Error> {
    // 0. get local binary directory.
    let binary_dir = get_binary_dir()?;
    log::info!("binary dir: {:?}", binary_dir);
    create_dir_all(&binary_dir)?;

    // 1. read and parse file list
    let task_list_str = include_str!("download-list.toml");
    let task_list: TaskList = toml::from_str(task_list_str)?;

    for task in &task_list.appimagetool {
        // 2. check file exists and file hash matches
        let filepath = Path::new(&binary_dir).join(&task.filename);
        if filepath.exists() {
            if let Ok(file_hash) = hashsum::sha256sum(&filepath, &hashsum::Options::default()) {
                if file_hash == task.sha256 {
                    log::info!("Skip exists file: {:?}", &filepath);
                    continue;
                } else {
                    log::error!(
                        "Hash mismatch, expected {:?}, got {:?}",
                        task.sha256,
                        &file_hash
                    );
                }
            }
        }

        for _retry in 0..3 {
            // 3. download file one by one with reqwest crate
            if let Err(err) = download_file(&task.url, &filepath) {
                log::error!("Failed to download {:?}, got error: {:?}", &task.url, err);
                continue;
            }

            // 4. check downloaded file hash
            match hashsum::sha256sum(&filepath, &hashsum::Options::default()) {
                Ok(file_hash) => {
                    if file_hash == task.sha256 {
                        break;
                    } else {
                        log::error!(
                            "Hash mismatch, expected {:?}, got {:?}",
                            task.sha256,
                            &file_hash
                        );
                    }
                }
                Err(err) => log::error!("err: {:?}", err),
            }
        }
    }

    Ok(())
}

fn download_file<P: AsRef<Path>>(url: &str, filepath: P) -> Result<(), Error> {
    log::info!("Downloading {} to {:?}", url, filepath.as_ref());
    let mut response = reqwest::blocking::get(url)?;
    let mut fd = File::create(filepath)?;
    std::io::copy(&mut response, &mut fd)
        .map(drop)
        .map_err(|err| err.into())
}
