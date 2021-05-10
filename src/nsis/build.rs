// Copyright (c) 2021 Xu Shaohua <shaohua@biofan.org>. All rights reserved.
// Use of this source is governed by General Public License that can be found
// in the LICENSE file.

use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

use super::config::NsisConfig;
use crate::base::Arch;
use crate::config::{Config, WindowsConfig};
use crate::BuildError;

pub fn build_nsis(
    conf: &Config,
    windows_conf: &WindowsConfig,
    arch: Arch,
) -> Result<(), BuildError> {
    let nsis_conf = if let Some(nsis_conf) = windows_conf.nsis.as_ref() {
        nsis_conf
    } else {
        return Err(BuildError::InvalidConfError);
    };

    if let Some(script) = nsis_conf.script.as_ref() {
        compile_nsis(&script)
    } else {
        let nsis_file = generate_nsis_file(conf, windows_conf, arch, nsis_conf)?;
        compile_nsis(&nsis_file)
    }
}

fn generate_nsis_file(
    conf: &Config,
    windows_conf: &WindowsConfig,
    arch: Arch,
    nsis_conf: &NsisConfig,
) -> Result<PathBuf, BuildError> {
    let files = if let Some(files) = nsis_conf.files.as_ref() {
        files
    } else if let Some(files) = windows_conf.files.as_ref() {
        files
    } else {
        return Err(BuildError::FilesNotSet);
    };

    let workdir = Path::new(&conf.metadata.workdir);
    let nsis_dir = workdir.join("nsis");
    fs::create_dir_all(&nsis_dir)?;
    let nsis_file = nsis_dir.join("app.nsi");
    let mut nsis_fd = File::create(&nsis_file)?;

    // Generate nsi script

    writeln!(nsis_fd, "# Generated by pifu. DO NOT EDIT!\n")?;
    writeln!(nsis_fd, "!include \"MUI2.nsh\"\n")?;

    if let Some(include_file) = nsis_conf.include.as_ref() {
        writeln!(nsis_fd, "!include {:?}\n", fs::canonicalize(include_file)?)?;
    }

    writeln!(nsis_fd, "Name {}", &conf.metadata.name)?;

    if nsis_conf.unicode {
        writeln!(nsis_fd, "Unicode True")?;
    } else {
        writeln!(nsis_fd, "Unicode False")?;
    }

    writeln!(nsis_fd, "OutFile \"{}\"", nsis_conf.artifact_name)?;
    writeln!(
        nsis_fd,
        "SetCompressor /SOLID {}\n",
        nsis_conf.compress_method
    )?;

    if nsis_conf.warnings_as_errors {
        writeln!(nsis_fd, "!define MUI_ABORTWARNING")?;
    }
    writeln!(
        nsis_fd,
        "!define MUI_ICON {:?}",
        fs::canonicalize(&nsis_conf.installer_icon)?
    )?;
    writeln!(
        nsis_fd,
        "!define MUI_UNICON {:?}",
        fs::canonicalize(&nsis_conf.uninstaller_icon)?
    )?;

    if nsis_conf.one_click {
        writeln!(
            nsis_fd,
            "InstallDir \"$LOCALAPPDATA\\Programs\\{}\"",
            &conf.metadata.name
        )?;
        writeln!(nsis_fd, "RequestExecutionlevel User")?;
        // Enable silent install.
        writeln!(nsis_fd, "SilentInstall silent")?;
    } else {
        if nsis_conf.per_machine {
            if arch == Arch::X86_64 {
                writeln!(
                    nsis_fd,
                    "InstallDir \"$PROGRAMFILES64\\{}\"",
                    &conf.metadata.name
                )?;
            } else {
                writeln!(nsis_fd, "InstallDir $PROGRAMFILES\\{}", &conf.metadata.name)?;
            }
            writeln!(nsis_fd, "RequestExecutionlevel Admin")?;
        } else {
            writeln!(
                nsis_fd,
                "InstallDir \"$LocalAppData\\Programs\\{}\"",
                &conf.metadata.name
            )?;
            writeln!(nsis_fd, "RequestExecutionlevel User")?;
        }

        writeln!(nsis_fd, "")?;

        if nsis_conf.allow_to_change_installation_directory {
            writeln!(nsis_fd, "!insertmacro MUI_PAGE_DIRECTORY")?;
        }
        writeln!(nsis_fd, "!insertmacro MUI_PAGE_INSTFILES")?;
        writeln!(nsis_fd, "!insertmacro MUI_UNPAGE_CONFIRM")?;
        writeln!(nsis_fd, "!insertmacro MUI_UNPAGE_INSTFILES")?;
    }

    writeln!(nsis_fd, "")?;
    writeln!(nsis_fd, "!insertmacro MUI_LANGUAGE \"English\"")?;

    let build_version = format!("{}.{}", &conf.metadata.version, &conf.metadata.build_id);

    // Version information.
    writeln!(nsis_fd, "VIProductVersion \"{}\"", &build_version)?;
    writeln!(nsis_fd, "VIFileVersion \"{}\"", &build_version)?;

    writeln!(
        nsis_fd,
        "VIAddVersionKey /LANG=${{LANG_ENGLISH}} \"ProductName\" \"{}\"",
        &conf.metadata.product_name
    )?;
    writeln!(
        nsis_fd,
        "VIAddVersionKey /LANG=${{LANG_ENGLISH}} \"ProductVersion\" \"{}\"",
        &conf.metadata.version
    )?;
    writeln!(
        nsis_fd,
        "VIAddVersionKey /LANG=${{LANG_ENGLISH}} \"FileDescription\" \"{}\"",
        &conf.metadata.description
    )?;
    if let Some(ref company) = conf.metadata.company {
        writeln!(
            nsis_fd,
            "VIAddVersionKey /LANG=${{LANG_ENGLISH}} \"CompanyName\" \"{}\"",
            company
        )?;
    }
    if let Some(ref copyright) = conf.metadata.copyright {
        writeln!(
            nsis_fd,
            "VIAddVersionKey /LANG=${{LANG_ENGLISH}} \"LegalCopyright\" \"{}\"",
            copyright
        )?;
    }
    writeln!(
        nsis_fd,
        "VIAddVersionKey /LANG=${{LANG_ENGLISH}} \"FileVersion\" \"{}\"",
        &build_version
    )?;

    // Install section
    writeln!(nsis_fd, "\nSection \"Install\"")?;
    writeln!(nsis_fd, "  SetOutPath \"$INSTDIR\"")?;
    let src = Path::new(".");
    for file in files {
        file.copy_to(&src, &nsis_dir)?;
        writeln!(nsis_fd, "  File {}", &file.to)?;
    }
    writeln!(nsis_fd, "  WriteUninstaller \"$INSTDIR\\Uninstall.exe\"")?;
    writeln!(nsis_fd, "SectionEnd")?;

    // Uninstall section
    writeln!(nsis_fd, "\nSection \"Uninstall\"")?;
    writeln!(nsis_fd, "  Delete \"$INSTDIR\\Uninstall.exe\"")?;
    writeln!(nsis_fd, "  RMDir /r \"$INSTDIR\"")?;
    writeln!(nsis_fd, "SectionEnd")?;

    Ok(nsis_file)
}

/// Compile nsis script
fn compile_nsis<P: AsRef<Path>>(nsis_file: &P) -> Result<(), BuildError> {
    let status = Command::new("makensis").arg(nsis_file.as_ref()).status()?;
    if status.success() {
        Ok(())
    } else {
        Err(BuildError::NsisCompilerError)
    }
}
