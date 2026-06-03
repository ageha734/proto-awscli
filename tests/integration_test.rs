use proto_pdk_test_utils::*;

mod awscli_tool {
    use super::*;

    #[tokio::test(flavor = "multi_thread")]
    async fn registers_tool_metadata() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("awscli-test").await;

        let output = plugin
            .register_tool(RegisterToolInput {
                id: "awscli-test".into(),
            })
            .await;

        assert_eq!(output.name, "AWS CLI");
        assert_eq!(output.type_of, PluginType::CommandLine);
        assert!(output.minimum_proto_version.is_some());
        assert!(output.plugin_version.is_some());
        assert_eq!(output.self_upgrade_commands, vec!["upgrade"]);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn loads_versions_from_git_tags() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("awscli-test").await;

        let output = plugin.load_versions(LoadVersionsInput::default()).await;

        assert!(!output.versions.is_empty());

        // All versions should be v2
        for version in &output.versions {
            let v = version.to_string();
            assert!(
                v.starts_with("2."),
                "Expected version to start with '2.', got: {}",
                v
            );
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn sets_latest_alias() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("awscli-test").await;

        let output = plugin.load_versions(LoadVersionsInput::default()).await;

        assert!(output.latest.is_some());
        assert!(output.aliases.contains_key("latest"));
        assert_eq!(output.aliases.get("latest"), output.latest.as_ref());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn resolves_v2_alias() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("awscli-test").await;

        let output = plugin
            .resolve_version(ResolveVersionInput {
                initial: UnresolvedVersionSpec::Alias("v2".into()),
                ..Default::default()
            })
            .await;

        assert!(output.candidate.is_some());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn downloads_linux_x64() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("awscli-test", |config| {
                config.host(HostOS::Linux, HostArch::X64);
            })
            .await;

        let output = plugin
            .download_prebuilt(DownloadPrebuiltInput {
                context: PluginContext {
                    version: VersionSpec::parse("2.22.0").unwrap(),
                    ..Default::default()
                },
                ..Default::default()
            })
            .await;

        assert_eq!(
            output.download_url,
            "https://awscli.amazonaws.com/awscli-exe-linux-x86_64-2.22.0.zip"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn downloads_linux_arm64() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("awscli-test", |config| {
                config.host(HostOS::Linux, HostArch::Arm64);
            })
            .await;

        let output = plugin
            .download_prebuilt(DownloadPrebuiltInput {
                context: PluginContext {
                    version: VersionSpec::parse("2.22.0").unwrap(),
                    ..Default::default()
                },
                ..Default::default()
            })
            .await;

        assert_eq!(
            output.download_url,
            "https://awscli.amazonaws.com/awscli-exe-linux-aarch64-2.22.0.zip"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn downloads_macos() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("awscli-test", |config| {
                config.host(HostOS::MacOS, HostArch::Arm64);
            })
            .await;

        let output = plugin
            .download_prebuilt(DownloadPrebuiltInput {
                context: PluginContext {
                    version: VersionSpec::parse("2.22.0").unwrap(),
                    ..Default::default()
                },
                ..Default::default()
            })
            .await;

        assert_eq!(
            output.download_url,
            "https://awscli.amazonaws.com/AWSCLIV2-2.22.0.pkg"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn downloads_windows() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("awscli-test", |config| {
                config.host(HostOS::Windows, HostArch::X64);
            })
            .await;

        let output = plugin
            .download_prebuilt(DownloadPrebuiltInput {
                context: PluginContext {
                    version: VersionSpec::parse("2.22.0").unwrap(),
                    ..Default::default()
                },
                ..Default::default()
            })
            .await;

        assert_eq!(
            output.download_url,
            "https://awscli.amazonaws.com/AWSCLIV2-2.22.0.msi"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn locates_linux_executables() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("awscli-test", |config| {
                config.host(HostOS::Linux, HostArch::X64);
            })
            .await;

        let output = plugin
            .locate_executables(LocateExecutablesInput {
                context: PluginContext {
                    version: VersionSpec::parse("2.22.0").unwrap(),
                    ..Default::default()
                },
                ..Default::default()
            })
            .await;

        let aws_exe = output.exes.get("aws").unwrap();
        assert_eq!(aws_exe.exe_path, Some("bin/aws".into()));
        assert!(aws_exe.primary);

        let completer_exe = output.exes.get("aws_completer").unwrap();
        assert_eq!(completer_exe.exe_path, Some("bin/aws_completer".into()));
        assert!(!completer_exe.primary);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn locates_macos_executables() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("awscli-test", |config| {
                config.host(HostOS::MacOS, HostArch::Arm64);
            })
            .await;

        let output = plugin
            .locate_executables(LocateExecutablesInput {
                context: PluginContext {
                    version: VersionSpec::parse("2.22.0").unwrap(),
                    ..Default::default()
                },
                ..Default::default()
            })
            .await;

        let aws_exe = output.exes.get("aws").unwrap();
        assert_eq!(aws_exe.exe_path, Some("aws-cli/aws".into()));
        assert!(aws_exe.primary);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn locates_windows_executables() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("awscli-test", |config| {
                config.host(HostOS::Windows, HostArch::X64);
            })
            .await;

        let output = plugin
            .locate_executables(LocateExecutablesInput {
                context: PluginContext {
                    version: VersionSpec::parse("2.22.0").unwrap(),
                    ..Default::default()
                },
                ..Default::default()
            })
            .await;

        let aws_exe = output.exes.get("aws").unwrap();
        assert_eq!(aws_exe.exe_path, Some("bin/aws.exe".into()));
        assert!(aws_exe.primary);
    }
}
