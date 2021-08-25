// Copyright (c) 2021 Xu Shaohua <shaohua@biofan.org>. All rights reserved.
// Use of this source is governed by General Public License that can be found
// in the LICENSE file.

use crate::app_image::build_app_image;
use crate::base::config::get_target_arch;
use crate::base::{Arch, PlatformTarget};
use crate::config::Config;
use crate::deb::build_deb;
use crate::nsis::build_nsis;
use crate::rpm::build_rpm;
use crate::Error;

#[derive(Debug)]
pub struct BuildOptions {
    pub ignore_error: bool,

    pub targets: Vec<PlatformTarget>,

    pub arches: Vec<Arch>,
}

impl Default for BuildOptions {
    fn default() -> Self {
        let arches = if let Some(arch) = get_target_arch() {
            vec![arch]
        } else {
            Vec::new()
        };

        Self {
            ignore_error: false,
            targets: vec![
                PlatformTarget::Deb,
                PlatformTarget::Rpm,
                PlatformTarget::AppImage,
                PlatformTarget::Nsis,
            ],
            arches,
        }
    }
}

pub fn build(conf: Config, options: &BuildOptions) -> Result<(), Error> {
    log::info!("build() conf: {:#?}", conf);

    if let Err(err) = build_linux(&conf, options) {
        if options.ignore_error {
            log::error!("build_linux() failed: {:?}", err);
        } else {
            return Err(err);
        }
    }

    build_windows(&conf)
}

fn build_linux(conf: &Config, options: &BuildOptions) -> Result<(), Error> {
    // Skip if `linux` section is not set.
    let linux_conf = if let Some(linux_conf) = conf.linux.as_ref() {
        linux_conf
    } else {
        return Ok(());
    };

    let arches = linux_conf
        .arch
        .iter()
        .filter(|a| options.arches.contains(a))
        .map(|a| *a)
        .collect::<Vec<Arch>>();
    let targets = linux_conf
        .targets
        .iter()
        .filter(|t| options.targets.contains(t))
        .map(|t| *t)
        .collect::<Vec<PlatformTarget>>();

    if targets.contains(&PlatformTarget::Deb) {
        for arch in &arches {
            build_deb(conf, linux_conf, *arch)?;
        }
    }
    if targets.contains(&PlatformTarget::Rpm) {
        for arch in &arches {
            build_rpm(conf, linux_conf, *arch)?;
        }
    }
    if targets.contains(&PlatformTarget::AppImage) {
        for arch in &arches {
            build_app_image(conf, linux_conf, *arch)?;
        }
    }

    Ok(())
}

fn build_windows(conf: &Config) -> Result<(), Error> {
    let windows_conf = if let Some(windows_conf) = conf.windows.as_ref() {
        windows_conf
    } else {
        return Ok(());
    };

    if windows_conf.targets.contains(&PlatformTarget::Nsis) {
        for arch in &windows_conf.arch {
            build_nsis(conf, windows_conf, *arch)?;
        }
    }

    Ok(())
}
