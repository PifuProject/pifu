// Copyright (c) 2021 Xu Shaohua <shaohua@biofan.org>. All rights reserved.
// Use of this source is governed by General Public License that can be found
// in the LICENSE file.

use std::path::Path;

use crate::base::fileset;
use crate::config::{Config, LinuxConfig};
use crate::BuildError;

pub fn build_deb(conf: &Config, linux_conf: &LinuxConfig) -> Result<(), BuildError> {
    let deb_conf = if let Some(deb_conf) = linux_conf.deb.as_ref() {
        deb_conf
    } else {
        // TODO(Shaohua): Returns error
        return Ok(());
    };

    let files = if let Some(files) = deb_conf.files.as_ref() {
        files
    } else if let Some(files) = linux_conf.files.as_ref() {
        files
    } else {
        return Err(BuildError::FilesNotSet);
    };

    let workdir = Path::new(&conf.metadata.workdir);
    let deb_dir = workdir.join("deb");
    let data_dir = deb_dir.join("data");
    let src_dir = Path::new(&conf.metadata.src_dir);

    fileset::copy_filesets(files, src_dir, &data_dir)?;

    Ok(())
}