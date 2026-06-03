use extism_pdk::*;
use proto_pdk::*;
use std::collections::HashMap;

#[host_fn]
extern "ExtismHost" {
    fn exec_command(input: Json<ExecCommandInput>) -> Json<ExecCommandOutput>;
}

static NAME: &str = "AWS CLI";

#[plugin_fn]
pub fn register_tool(Json(_): Json<RegisterToolInput>) -> FnResult<Json<RegisterToolOutput>> {
    Ok(Json(RegisterToolOutput {
        name: NAME.into(),
        type_of: PluginType::CommandLine,
        minimum_proto_version: Some(Version::new(0, 46, 0)),
        plugin_version: Version::parse(env!("CARGO_PKG_VERSION")).ok(),
        self_upgrade_commands: vec!["upgrade".into()],
        ..RegisterToolOutput::default()
    }))
}

#[plugin_fn]
pub fn load_versions(Json(_): Json<LoadVersionsInput>) -> FnResult<Json<LoadVersionsOutput>> {
    let tags = load_git_tags("https://github.com/aws/aws-cli")?
        .into_iter()
        .filter_map(|tag| {
            // Only include v2 tags (e.g., "2.x.x")
            if tag.starts_with("2.") {
                Some(tag)
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    Ok(Json(LoadVersionsOutput::from(tags)?))
}

#[plugin_fn]
pub fn resolve_version(
    Json(input): Json<ResolveVersionInput>,
) -> FnResult<Json<ResolveVersionOutput>> {
    let mut output = ResolveVersionOutput::default();

    // Map "v2" alias to latest v2 range
    if let UnresolvedVersionSpec::Alias(alias) = &input.initial {
        if alias == "v2" || alias == "2" {
            output.candidate = Some(UnresolvedVersionSpec::parse(">=2.0.0")?);
        }
    }

    Ok(Json(output))
}

#[plugin_fn]
pub fn download_prebuilt(
    Json(input): Json<DownloadPrebuiltInput>,
) -> FnResult<Json<DownloadPrebuiltOutput>> {
    let env = get_host_environment()?;
    let version = &input.context.version;

    if version.is_canary() {
        return Err(plugin_err!(PluginError::UnsupportedCanary {
            tool: NAME.into()
        }));
    }

    check_supported_os_and_arch(
        NAME,
        &env,
        permutations![
            HostOS::Linux => [HostArch::X64, HostArch::Arm64],
            HostOS::MacOS => [HostArch::X64, HostArch::Arm64],
            HostOS::Windows => [HostArch::X64],
        ],
    )?;

    let version_str = version.to_string();

    let download_url = match env.os {
        HostOS::Linux => {
            let arch = match env.arch {
                HostArch::Arm64 => "aarch64",
                _ => "x86_64",
            };
            format!(
                "https://awscli.amazonaws.com/awscli-exe-linux-{arch}-{version_str}.zip"
            )
        }
        HostOS::MacOS => {
            format!(
                "https://awscli.amazonaws.com/AWSCLIV2-{version_str}.pkg"
            )
        }
        HostOS::Windows => {
            format!(
                "https://awscli.amazonaws.com/AWSCLIV2-{version_str}.msi"
            )
        }
        _ => unreachable!(),
    };

    let download_name = match env.os {
        HostOS::Linux => {
            let arch = match env.arch {
                HostArch::Arm64 => "aarch64",
                _ => "x86_64",
            };
            Some(format!("awscli-exe-linux-{arch}-{version_str}.zip"))
        }
        HostOS::MacOS => Some(format!("AWSCLIV2-{version_str}.pkg")),
        HostOS::Windows => Some(format!("AWSCLIV2-{version_str}.msi")),
        _ => None,
    };

    Ok(Json(DownloadPrebuiltOutput {
        download_url,
        download_name,
        ..DownloadPrebuiltOutput::default()
    }))
}

/// Download a file using curl on the host machine.
fn download_file(url: &str, dest: &str) -> Result<(), Error> {
    let output = exec(ExecCommandInput {
        command: "curl".into(),
        args: vec![
            "-fSL".into(),
            "--create-dirs".into(),
            "-o".into(),
            dest.into(),
            url.into(),
        ],
        stream: true,
        ..ExecCommandInput::default()
    })?;

    if output.exit_code != 0 {
        return Err(plugin_err!(
            "Failed to download {}: {}",
            url,
            output.stderr
        ));
    }

    Ok(())
}

#[plugin_fn]
pub fn native_install(
    Json(input): Json<NativeInstallInput>,
) -> FnResult<Json<NativeInstallOutput>> {
    let env = get_host_environment()?;
    let install_dir = &input.install_dir;
    let version = &input.context.version;
    let version_str = version.to_string();

    match env.os {
        HostOS::Linux => {
            let arch = match env.arch {
                HostArch::Arm64 => "aarch64",
                _ => "x86_64",
            };

            let zip_name = format!("awscli-exe-linux-{arch}-{version_str}.zip");
            let zip_url = format!("https://awscli.amazonaws.com/{zip_name}");
            let temp_dir = &input.context.temp_dir;
            let zip_path = temp_dir.join(&zip_name);

            // Download the zip archive using curl on the host
            debug!("Downloading AWS CLI from <url>{}</url>", zip_url);

            download_file(&zip_url, &zip_path.to_string_lossy())?;

            // Unzip the archive
            debug!("Extracting AWS CLI archive");

            let unzip_output = exec_captured(
                "unzip",
                [
                    "-o",
                    &zip_path.to_string_lossy(),
                    "-d",
                    &temp_dir.to_string_lossy(),
                ],
            )?;

            if unzip_output.exit_code != 0 {
                return Ok(Json(NativeInstallOutput {
                    installed: false,
                    error: Some(format!(
                        "Failed to unzip AWS CLI archive: {}",
                        unzip_output.stderr
                    )),
                    ..NativeInstallOutput::default()
                }));
            }

            // Run the installer with --install-dir pointing to proto's install directory
            let installer_path = temp_dir.join("aws").join("install");
            let bin_dir = install_dir.join("bin");

            debug!(
                "Running AWS CLI installer to <path>{}</path>",
                install_dir.display()
            );

            let install_output = exec(ExecCommandInput {
                command: installer_path.to_string_lossy().to_string(),
                args: vec![
                    "--install-dir".into(),
                    install_dir.to_string_lossy().to_string(),
                    "--bin-dir".into(),
                    bin_dir.to_string_lossy().to_string(),
                    "--update".into(),
                ],
                set_executable: true,
                stream: true,
                ..ExecCommandInput::default()
            })?;

            if install_output.exit_code != 0 {
                return Ok(Json(NativeInstallOutput {
                    installed: false,
                    error: Some(format!(
                        "AWS CLI installer failed: {}",
                        install_output.stderr
                    )),
                    ..NativeInstallOutput::default()
                }));
            }
        }
        HostOS::MacOS => {
            let pkg_name = format!("AWSCLIV2-{version_str}.pkg");
            let pkg_url = format!("https://awscli.amazonaws.com/{pkg_name}");
            let temp_dir = &input.context.temp_dir;
            let pkg_path = temp_dir.join(&pkg_name);

            // Download the pkg using curl on the host
            debug!("Downloading AWS CLI from <url>{}</url>", pkg_url);

            download_file(&pkg_url, &pkg_path.to_string_lossy())?;

            // Use pkgutil to expand the pkg, then install to the proto directory
            let expanded_dir = temp_dir.join("aws-cli-expanded");

            debug!("Expanding AWS CLI package");

            let expand_output = exec_captured(
                "pkgutil",
                [
                    "--expand-full",
                    &pkg_path.to_string_lossy(),
                    &expanded_dir.to_string_lossy(),
                ],
            )?;

            if expand_output.exit_code != 0 {
                return Ok(Json(NativeInstallOutput {
                    installed: false,
                    error: Some(format!(
                        "Failed to expand AWS CLI package: {}",
                        expand_output.stderr
                    )),
                    ..NativeInstallOutput::default()
                }));
            }

            // The expanded pkg contains aws-cli.pkg/Payload/aws-cli/
            // Copy the contents to the install directory
            let payload_dir = expanded_dir
                .join("aws-cli.pkg")
                .join("Payload")
                .join("aws-cli");

            debug!(
                "Copying AWS CLI to <path>{}</path>",
                install_dir.display()
            );

            let copy_output = exec_captured(
                "cp",
                [
                    "-R",
                    &format!("{}/.", payload_dir.to_string_lossy()),
                    &install_dir.to_string_lossy(),
                ],
            )?;

            if copy_output.exit_code != 0 {
                return Ok(Json(NativeInstallOutput {
                    installed: false,
                    error: Some(format!(
                        "Failed to copy AWS CLI files: {}",
                        copy_output.stderr
                    )),
                    ..NativeInstallOutput::default()
                }));
            }
        }
        HostOS::Windows => {
            let msi_name = format!("AWSCLIV2-{version_str}.msi");
            let msi_url = format!("https://awscli.amazonaws.com/{msi_name}");
            let temp_dir = &input.context.temp_dir;
            let msi_path = temp_dir.join(&msi_name);

            // Download the MSI using curl on the host
            debug!("Downloading AWS CLI from <url>{}</url>", msi_url);

            download_file(&msi_url, &msi_path.to_string_lossy())?;

            // Install using msiexec with target directory
            debug!(
                "Installing AWS CLI to <path>{}</path>",
                install_dir.display()
            );

            let install_output = exec(ExecCommandInput {
                command: "msiexec".into(),
                args: vec![
                    "/i".into(),
                    msi_path.to_string_lossy().to_string(),
                    "/qn".into(),
                    format!("INSTALLDIR={}", install_dir.to_string_lossy()),
                ],
                stream: true,
                ..ExecCommandInput::default()
            })?;

            if install_output.exit_code != 0 {
                return Ok(Json(NativeInstallOutput {
                    installed: false,
                    error: Some(format!(
                        "AWS CLI MSI installation failed: {}",
                        install_output.stderr
                    )),
                    ..NativeInstallOutput::default()
                }));
            }
        }
        _ => {
            return Ok(Json(NativeInstallOutput {
                installed: false,
                error: Some(format!("Unsupported operating system: {}", env.os)),
                ..NativeInstallOutput::default()
            }));
        }
    }

    Ok(Json(NativeInstallOutput {
        installed: true,
        ..NativeInstallOutput::default()
    }))
}

#[plugin_fn]
pub fn native_uninstall(
    Json(input): Json<NativeUninstallInput>,
) -> FnResult<Json<NativeUninstallOutput>> {
    let env = get_host_environment()?;

    match env.os {
        HostOS::Windows => {
            // On Windows, use msiexec to uninstall
            let _ = exec_captured("msiexec", ["/x", "AWSCLIV2", "/qn"]);
        }
        _ => {
            // On Linux/macOS, proto handles directory removal
            debug!(
                "Removing AWS CLI from <path>{}</path>",
                input.uninstall_dir.display()
            );
        }
    }

    Ok(Json(NativeUninstallOutput {
        uninstalled: true,
        ..NativeUninstallOutput::default()
    }))
}

#[plugin_fn]
pub fn locate_executables(
    Json(_): Json<LocateExecutablesInput>,
) -> FnResult<Json<LocateExecutablesOutput>> {
    let env = get_host_environment()?;

    let exe_path = match env.os {
        HostOS::Windows => "bin/aws.exe".to_string(),
        HostOS::MacOS => "aws-cli/aws".to_string(),
        _ => "bin/aws".to_string(),
    };

    let completer_path = match env.os {
        HostOS::Windows => "bin/aws_completer.exe".to_string(),
        HostOS::MacOS => "aws-cli/aws_completer".to_string(),
        _ => "bin/aws_completer".to_string(),
    };

    Ok(Json(LocateExecutablesOutput {
        exes: HashMap::from_iter([
            (
                "aws".into(),
                ExecutableConfig::new_primary(&exe_path),
            ),
            (
                "aws_completer".into(),
                ExecutableConfig::new(&completer_path),
            ),
        ]),
        ..LocateExecutablesOutput::default()
    }))
}
