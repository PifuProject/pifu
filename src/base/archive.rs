// Copyright (c) 2021 Xu Shaohua <shaohua@biofan.org>. All rights reserved.
// Use of this source is governed by General Public License that can be found
// in the LICENSE file.

use std::fs::{self, File};
use std::io;
#[cfg(not(target_os = "windows"))]
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::time::UNIX_EPOCH;
use walkdir::WalkDir;

use crate::BuildError;

pub fn create_tar(dir: &Path, to: &Path) -> Result<(), BuildError> {
    log::info!("tar {:?} > {:?}", dir, to);
    let to_file = File::create(to)?;
    let mut builder = tar::Builder::new(to_file);
    builder.append_dir_all(".", dir)?;
    builder.finish()?;

    Ok(())
}

pub fn create_tar_chown(dir: &Path, to: &Path) -> Result<(), BuildError> {
    log::info!("tar {:?} > {:?}", dir, to);
    let to_file = File::create(to)?;
    let mut builder = tar::Builder::new(to_file);

    for entry in WalkDir::new(dir) {
        let entry = entry?;
        let path = entry.path();

        let metadata = fs::metadata(path)?;
        let mtime = metadata.modified()?;
        let mtime = mtime.duration_since(UNIX_EPOCH)?.as_secs();
        let filename = path.strip_prefix(dir)?;

        #[cfg(not(target_os = "windows"))]
        let mode = metadata.permissions().mode();

        if path.is_file() {
            let mut header = tar::Header::new_gnu();
            header.set_mtime(mtime);
            header.set_size(metadata.len());

            #[cfg(not(target_os = "windows"))]
            header.set_mode(mode);

            let path_str = filename.to_string_lossy().to_string();
            header.set_path(&path_str)?;
            header.set_cksum();
            let fd = File::open(path)?;
            builder.append(&mut header, fd)?;
        } else if path.is_dir() {
            let mut header = tar::Header::new_gnu();
            header.set_mtime(mtime);
            header.set_size(0);
            header.set_mode(0o755);
            if filename.as_os_str().is_empty() {
                continue;
            }
            let dir_path = Path::new("./.").join(filename);
            let mut path_str = dir_path.to_string_lossy().to_string();
            if !path_str.ends_with('/') {
                path_str += "/";
            }
            header.set_path(&filename)?;
            header.set_entry_type(tar::EntryType::Directory);
            header.set_cksum();
            builder.append(&mut header, &mut io::empty())?;
        }
    }

    builder.finish()?;

    Ok(())
}

pub fn create_ar(dir: &Path, to: &Path) -> Result<(), BuildError> {
    log::info!("ar {:?} > {:?}", dir, to);
    let to_file = File::create(to)?;
    let mut builder = ar::Builder::new(to_file);

    for entry in WalkDir::new(dir) {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            builder.append_path(path)?;
        }
    }

    Ok(())
}

pub fn create_ar_files<P: AsRef<Path>>(files: &[&P], to: &Path) -> Result<(), BuildError> {
    let to_file = File::create(to)?;
    let mut builder = ar::Builder::new(to_file);

    for path in files {
        let path = path.as_ref();
        if path.is_file() {
            builder.append_path(path)?;
        }
    }

    Ok(())
}
