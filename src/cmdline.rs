// Copyright (c) 2021 Xu Shaohua <shaohua@biofan.org>. All rights reserved.
// Use of this source is governed by General Public License that can be found
// in the LICENSE file.

use clap::{App, Arg};
use std::fs;
use std::path::Path;

use crate::base::{expand_file_macro_simple, Arch, PlatformTarget};
use crate::build;
use crate::config::Config;
use crate::download;
use crate::error::{Error, ErrorKind};

pub fn read_cmdline() -> Result<(), Error> {
    let matches = App::new("Pifu package builder")
        .version("0.2.4")
        .author("Xu Shaohua <shaohua@biofan.org>")
        .about("General package builder")
        .arg(
            Arg::with_name("config")
                .short("c")
                .long("config")
                .value_name("toml file")
                .help("Specify a custom toml config file")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("os")
                .long("os")
                .multiple(true)
                .help("Build specific OS platforms")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("download")
                .long("download")
                .help("Download required tools from github")
                .takes_value(false),
        )
        .get_matches();

    if matches.is_present("download") {
        return download::download();
    }

    // read config
    let mut config_file = matches.value_of("config").unwrap_or("pkg/pifu.toml");
    if !Path::new(config_file).exists() {
        config_file = "pifu.toml";
    }
    log::info!("config file: {:?}", config_file);

    let config_content = fs::read_to_string(config_file)
        .expect(&format!("Failed to read config at {}", config_file));
    let mut conf: Config = toml::from_str(&config_content).expect("Invalid config");

    conf.metadata.build_id = expand_file_macro_simple(&conf.metadata.build_id)?;

    let mut options = build::BuildOptions::default();
    if let Some(os_list) = matches.values_of("os") {
        options.targets.clear();
        for os in os_list {
            if os == "linux" {
                options.targets.extend([
                    PlatformTarget::Deb,
                    PlatformTarget::Rpm,
                    PlatformTarget::AppImage,
                ]);
            } else if os == "win" {
                options.targets.push(PlatformTarget::Nsis);
            } else {
                log::error!("Invalid --os {}", &os);
                return Err(Error::from_string(
                    ErrorKind::CmdlineError,
                    format!("Invalid --os {}, available values are `linux` or `win`", os),
                ));
            }
        }
    }

    log::info!("options: {:#?}", options);
    //build::build(conf, &options)
    Ok(())
}
