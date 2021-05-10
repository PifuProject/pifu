// Copyright (c) 2021 Xu Shaohua <shaohua@biofan.org>. All rights reserved.
// Use of this source is governed by General Public License that can be found
// in the LICENSE file.

use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

use super::config::NsisConfig;
use crate::base::{expand_file_macro, Arch, PlatformTarget};
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

    let artifact_name =
        expand_file_macro(&nsis_conf.artifact_name, conf, arch, PlatformTarget::Nsis)?;
    writeln!(nsis_fd, r#"OutFile "{}""#, artifact_name)?;
    writeln!(
        nsis_fd,
        "SetCompressor /SOLID {}\n",
        nsis_conf.compress_method
    )?;

    if nsis_conf.warnings_as_errors {
        writeln!(nsis_fd, "!define MUI_ABORTWARNING")?;
    }

    // Icons
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

    if let Some(header_icon) = nsis_conf.installer_header_icon.as_ref() {
        writeln!(nsis_fd, "!define MUI_HEADERIMAGE")?;
        writeln!(
            nsis_fd,
            "!define MUI_HEADERIMAGE_BITMAP {:?}",
            fs::canonicalize(header_icon)?
        )?;
    }
    if let Some(installer_sidebar) = nsis_conf.installer_sidebar.as_ref() {
        writeln!(
            nsis_fd,
            "!define MUI_WELCOMEFINISHPAGE_BITMAP {:?}",
            fs::canonicalize(installer_sidebar)?
        )?;
    }
    if let Some(uninstaller_sidebar) = nsis_conf.uninstaller_sidebar.as_ref() {
        writeln!(
            nsis_fd,
            "!define MUI_UNWELCOMEFINISHPAGE_BITMAP {:?}",
            fs::canonicalize(uninstaller_sidebar)?
        )?;
    }

    // Setup pages
    if nsis_conf.one_click {
        writeln!(
            nsis_fd,
            r#"InstallDir "$LOCALAPPDATA\Programs\{}""#,
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
                    r#"InstallDir "$PROGRAMFILES64\{}""#,
                    &conf.metadata.name
                )?;
            } else {
                writeln!(
                    nsis_fd,
                    r#"InstallDir $PROGRAMFILES\{}"#,
                    &conf.metadata.name
                )?;
            }
            writeln!(nsis_fd, "RequestExecutionlevel Admin")?;
        } else {
            writeln!(
                nsis_fd,
                r#"InstallDir "$LocalAppData\Programs\{}""#,
                &conf.metadata.name
            )?;
            writeln!(nsis_fd, "RequestExecutionlevel User")?;
        }

        writeln!(nsis_fd, "")?;

        if nsis_conf.installer_sidebar.is_some() {
            writeln!(nsis_fd, "!insertmacro MUI_PAGE_WELCOME")?;
        }

        if let Some(license_file) = conf.metadata.license_file.as_ref() {
            writeln!(
                nsis_fd,
                "!insertmacro MUI_PAGE_LICENSE {:?}",
                fs::canonicalize(license_file)?
            )?;
        }

        if nsis_conf.allow_to_change_installation_directory {
            writeln!(nsis_fd, "!insertmacro MUI_PAGE_DIRECTORY")?;
        }

        writeln!(nsis_fd, "!insertmacro MUI_PAGE_INSTFILES\n")?;

        if nsis_conf.run_after_finish {
            writeln!(
                nsis_fd,
                r#"!define MUI_FINISHPAGE_RUN "$INSTDIR\{}""#,
                &windows_conf.exe_file
            )?;
            writeln!(nsis_fd, "!define MUI_FINISHPAGE_NOREBOOTSUPPORT")?;
            writeln!(nsis_fd, "!insertmacro MUI_PAGE_FINISH\n")?;
        }

        if nsis_conf.uninstaller_sidebar.is_some() {
            writeln!(nsis_fd, "!insertmacro MUI_UNPAGE_WELCOME")?;
        }
        writeln!(nsis_fd, "!insertmacro MUI_UNPAGE_CONFIRM")?;
        writeln!(nsis_fd, "!insertmacro MUI_UNPAGE_INSTFILES")?;
    }

    writeln!(nsis_fd, "")?;
    writeln!(nsis_fd, r#"!insertmacro MUI_LANGUAGE "English""#)?;

    let build_version = format!("{}.{}", &conf.metadata.version, &conf.metadata.build_id);

    // Version information.
    writeln!(nsis_fd, r#"VIProductVersion "{}""#, &build_version)?;
    writeln!(nsis_fd, r#"VIFileVersion "{}""#, &build_version)?;

    writeln!(
        nsis_fd,
        r#"VIAddVersionKey /LANG=${{LANG_ENGLISH}} "ProductName" "{}""#,
        &conf.metadata.product_name
    )?;
    writeln!(
        nsis_fd,
        r#"VIAddVersionKey /LANG=${{LANG_ENGLISH}} "ProductVersion" "{}""#,
        &conf.metadata.version
    )?;
    writeln!(
        nsis_fd,
        r#"VIAddVersionKey /LANG=${{LANG_ENGLISH}} "FileDescription" "{}""#,
        &conf.metadata.description
    )?;
    if let Some(ref company) = conf.metadata.company {
        writeln!(
            nsis_fd,
            r#"VIAddVersionKey /LANG=${{LANG_ENGLISH}} "CompanyName" "{}""#,
            company
        )?;
    }
    if let Some(ref copyright) = conf.metadata.copyright {
        writeln!(
            nsis_fd,
            r#"VIAddVersionKey /LANG=${{LANG_ENGLISH}} "LegalCopyright" "{}""#,
            copyright
        )?;
    }
    writeln!(
        nsis_fd,
        r#"VIAddVersionKey /LANG=${{LANG_ENGLISH}} "FileVersion" "{}""#,
        &build_version
    )?;

    // Install section
    writeln!(nsis_fd, "\nSection \"Install\"")?;
    writeln!(nsis_fd, r#"  SetOutPath "$INSTDIR""#)?;
    let src = Path::new(".");
    for file in files {
        file.copy_to(&src, &nsis_dir)?;
        writeln!(nsis_fd, "  File {}", &file.to)?;
    }
    writeln!(nsis_fd, r#"  WriteUninstaller "$INSTDIR\Uninstall.exe""#)?;

    let reg_section = if nsis_conf.per_machine {
        "HKLM"
    } else {
        "HKCU"
    };

    let reg_uninst_key = format!(
        r#"Software\Microsoft\Windows\CurrentVersion\Uninstall\{}"#,
        &conf.metadata.product_name
    );

    writeln!(
        nsis_fd,
        r#"  WriteRegStr {} "{}" "UninstallString" '"$INSTDIR\Uninstall.exe"'"#,
        reg_section, reg_uninst_key
    )?;
    writeln!(
        nsis_fd,
        r#"  WriteRegStr {} "{}" "QuietUninstallString" '"$INSTDIR\Uninstall.exe" /S'"#,
        reg_section, reg_uninst_key
    )?;
    writeln!(
        nsis_fd,
        r#"  WriteRegStr {} "{}" "InstallLocation" "$INSTDIR""#,
        reg_section, reg_uninst_key
    )?;
    writeln!(
        nsis_fd,
        r#"  WriteRegStr {} "{}" "DisplayName" "{}""#,
        reg_section, reg_uninst_key, &conf.metadata.product_name
    )?;
    writeln!(
        nsis_fd,
        r#"WriteRegStr {} "{}" "DisplayIcon" "$INSTDIR\Uninstall.exe,0""#,
        reg_section, reg_uninst_key
    )?;
    writeln!(
        nsis_fd,
        r#"WriteRegStr {} "{}" "DisplayVersion" "{}""#,
        reg_section, reg_uninst_key, &conf.metadata.version
    )?;
    if let Some(company) = conf.metadata.company.as_ref() {
        writeln!(
            nsis_fd,
            r#"WriteRegStr {} "{}" "Publisher" "{}""#,
            reg_section, reg_uninst_key, company
        )?;
    }
    writeln!(
        nsis_fd,
        r#"  WriteRegDWORD {} "{}" "NoModify" "1""#,
        reg_section, reg_uninst_key
    )?;
    writeln!(
        nsis_fd,
        r#"  WriteRegDWORD {} "{}" "NoRepair" "1""#,
        reg_section, reg_uninst_key
    )?;
    //WriteRegStr HKLM "${REG_UNINST_KEY}" "InstallDate" $1

    if nsis_conf.run_on_startup {
        writeln!(
            nsis_fd,
            r#"  WriteRegStr {} "Software\Microsoft\Windows\CurrentVersion\Run" "{}" '"$INSTDIR\{}"'"#,
            reg_section, &conf.metadata.product_name, &windows_conf.exe_file
        )?;
    }

    if nsis_conf.create_start_menu_shortcut {
        writeln!(
            nsis_fd,
            r#"  CreateShortcut "$SMPROGRAMS\{}.lnk" "$INSTDIR\{}""#,
            &conf.metadata.product_name, &windows_conf.exe_file
        )?;
    }

    writeln!(nsis_fd, "SectionEnd")?;

    // Uninstall section
    writeln!(nsis_fd, "\nSection \"Uninstall\"")?;
    writeln!(nsis_fd, r#"  Delete "$INSTDIR\Uninstall.exe""#)?;
    writeln!(nsis_fd, r#"  RMDir /r "$INSTDIR""#)?;
    if nsis_conf.run_on_startup {
        writeln!(
            nsis_fd,
            r#"  DeleteRegKey {} "Software\Microsoft\Windows\CurrentVersion\Run\{}""#,
            reg_section, &conf.metadata.product_name,
        )?;
    }
    writeln!(
        nsis_fd,
        r#"  DeleteRegKey {} "{}""#,
        reg_section, reg_uninst_key
    )?;
    if nsis_conf.create_start_menu_shortcut {
        writeln!(
            nsis_fd,
            r#"  Delete "$SMPROGRAMS\{}.lnk""#,
            &conf.metadata.product_name
        )?;
    }
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
