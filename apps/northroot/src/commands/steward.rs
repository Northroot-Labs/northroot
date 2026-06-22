//! Steward custody profile CLI wrapper.

use clap::Subcommand;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Steward custody profile subcommands.
#[derive(Subcommand)]
pub enum StewardCommand {
    /// Initialize steward profile files from custody inventory and policy
    Init {
        /// Custody workspace inventory JSON
        #[arg(long)]
        inventory: PathBuf,
        /// Custody policy JSON
        #[arg(long)]
        policy: PathBuf,
        /// Output state directory
        #[arg(long)]
        output: PathBuf,
        /// Steward profile name
        #[arg(long, default_value = "steward")]
        profile_name: String,
        /// Private secret binding JSON for unattended delegated runs
        #[arg(long)]
        secret_bindings: Option<PathBuf>,
        /// Private repository binding JSON for runnable repository targets
        #[arg(long)]
        repository_bindings: Option<PathBuf>,
    },
    /// Report steward profile status
    Status {
        /// Steward state directory
        #[arg(long)]
        state: PathBuf,
    },
    /// Check delegated tool and unattended secret readiness
    Preflight {
        /// Steward state directory
        #[arg(long)]
        state: PathBuf,
    },
    /// Verify steward state readiness without side effects
    VerifyState {
        /// Steward state directory
        #[arg(long)]
        state: PathBuf,
        /// Snapshot id used to evaluate recorded retention evidence
        #[arg(long)]
        snapshot_id: Option<String>,
    },
    /// Print agent-safe steward capability manifest
    Capabilities {
        /// Steward state directory
        #[arg(long)]
        state: PathBuf,
    },
    /// Plan a constrained agent-safe steward argv without executing it
    CommandPlan {
        /// Steward state directory
        #[arg(long)]
        state: PathBuf,
        /// Steward operation to plan
        #[arg(long, value_parser = [
            "status",
            "preflight",
            "verify-state",
            "capabilities",
            "report",
            "run",
            "verify",
            "restore",
            "restore-drill",
            "schedule.create",
            "schedule.status",
            "schedule.install",
            "schedule.uninstall",
            "schedule.delete",
            "retention.evaluate",
            "evidence.report",
            "evidence.record",
            "offsite.report",
        ])]
        operation: String,
        /// Snapshot id to bind in the planned argv
        #[arg(long)]
        snapshot_id: Option<String>,
        /// Restore target to bind in the planned argv
        #[arg(long)]
        target: Option<PathBuf>,
        /// Include --execute in the planned argv
        #[arg(long)]
        execute: bool,
        /// Scheduler backend for schedule.create
        #[arg(long, value_parser = ["launchd", "systemd"])]
        scheduler: Option<String>,
        /// Scheduled steward operation for schedule.create
        #[arg(long, value_parser = ["run", "verify", "restore-drill"])]
        schedule_operation: Option<String>,
        /// Schedule interval in minutes for schedule.create
        #[arg(long)]
        every_minutes: Option<u64>,
        /// Runner command for schedule.create
        #[arg(long)]
        runner_command: Option<String>,
        /// Evidence item to bind in the planned argv; repeat for multiple entries
        #[arg(long, value_parser = ["verified_offsite_copy"])]
        evidence: Vec<String>,
        /// External evidence source for evidence.record
        #[arg(long)]
        source: Option<String>,
        /// Human-readable evidence detail for evidence.record
        #[arg(long)]
        detail: Option<String>,
        /// Private artifact reference for evidence.record
        #[arg(long)]
        artifact_ref: Option<String>,
        /// Include --force in the planned argv
        #[arg(long)]
        force: bool,
        /// Include --use-recorded-evidence in the planned argv
        #[arg(long)]
        use_recorded_evidence: bool,
        /// Include --skip-preflight in the planned argv
        #[arg(long)]
        skip_preflight: bool,
    },
    /// Render a consolidated read-only custody report
    Report {
        /// Steward state directory
        #[arg(long)]
        state: PathBuf,
        /// Snapshot id used to include snapshot-scoped evidence and retention state
        #[arg(long)]
        snapshot_id: Option<String>,
    },
    /// Report evidence derived from steward run summaries
    Evidence {
        #[command(subcommand)]
        command: StewardEvidenceCommand,
    },
    /// Report externally delegated offsite copy requirements
    Offsite {
        #[command(subcommand)]
        command: StewardOffsiteCommand,
    },
    /// Render the delegated backup run command
    Run {
        /// Steward state directory
        #[arg(long)]
        state: PathBuf,
        /// Snapshot id this run evidence should be bound to
        #[arg(long)]
        snapshot_id: Option<String>,
        /// Execute the delegated resticprofile command
        #[arg(long)]
        execute: bool,
    },
    /// Render the delegated verification command
    Verify {
        /// Steward state directory
        #[arg(long)]
        state: PathBuf,
        /// Snapshot id this verification evidence should be bound to
        #[arg(long)]
        snapshot_id: Option<String>,
        /// Execute the delegated resticprofile command
        #[arg(long)]
        execute: bool,
    },
    /// Render or execute a delegated recovery restore
    Restore {
        /// Steward state directory
        #[arg(long)]
        state: PathBuf,
        /// Snapshot id to restore
        #[arg(long)]
        snapshot_id: String,
        /// Recovery restore target directory
        #[arg(long)]
        target: PathBuf,
        /// Execute the delegated resticprofile restore command
        #[arg(long)]
        execute: bool,
    },
    /// Render or execute a delegated restore drill
    RestoreDrill {
        /// Steward state directory
        #[arg(long)]
        state: PathBuf,
        /// Restore drill target directory
        #[arg(long)]
        target: Option<PathBuf>,
        /// Snapshot id this restore evidence should be bound to
        #[arg(long)]
        snapshot_id: Option<String>,
        /// Execute the delegated resticprofile restore command
        #[arg(long)]
        execute: bool,
    },
    /// Render delegated scheduler templates
    Schedule {
        #[command(subcommand)]
        command: StewardScheduleCommand,
    },
    /// Evaluate retention gates before prune/offload work
    Retention {
        #[command(subcommand)]
        command: StewardRetentionCommand,
    },
}

/// Steward scheduler template subcommands.
#[derive(Subcommand)]
pub enum StewardScheduleCommand {
    /// Create a scheduler template without installing it
    Create {
        /// Steward state directory
        #[arg(long)]
        state: PathBuf,
        /// Scheduler backend to render
        #[arg(long, value_parser = ["launchd", "systemd"])]
        scheduler: String,
        /// Steward operation to execute on the schedule
        #[arg(long, default_value = "run", value_parser = ["run", "verify", "restore-drill"])]
        operation: String,
        /// Schedule interval in minutes
        #[arg(long)]
        every_minutes: u64,
        /// Runner command embedded in the generated scheduler template
        #[arg(long, default_value = "nr steward")]
        runner_command: String,
    },
    /// Report generated schedule template status
    Status {
        /// Steward state directory
        #[arg(long)]
        state: PathBuf,
    },
    /// Install generated scheduler templates through the platform scheduler
    Install {
        /// Steward state directory
        #[arg(long)]
        state: PathBuf,
        /// Execute launchctl/systemctl commands instead of only rendering them
        #[arg(long)]
        execute: bool,
        /// Install without checking whether scheduled steward execution is ready
        #[arg(long)]
        skip_preflight: bool,
    },
    /// Uninstall generated scheduler templates through the platform scheduler
    Uninstall {
        /// Steward state directory
        #[arg(long)]
        state: PathBuf,
        /// Execute launchctl/systemctl commands instead of only rendering them
        #[arg(long)]
        execute: bool,
    },
    /// Delete generated scheduler templates
    Delete {
        /// Steward state directory
        #[arg(long)]
        state: PathBuf,
        /// Delete generated files even when the schedule is marked installed
        #[arg(long)]
        force: bool,
    },
}

/// Steward offsite copy report subcommands.
#[derive(Subcommand)]
pub enum StewardOffsiteCommand {
    /// Report externally delegated offsite copy requirements for a snapshot
    Report {
        /// Steward state directory
        #[arg(long)]
        state: PathBuf,
        /// Snapshot id being checked for offsite evidence
        #[arg(long)]
        snapshot_id: String,
    },
}

/// Steward evidence subcommands.
#[derive(Subcommand)]
pub enum StewardEvidenceCommand {
    /// Report evidence derived from steward run summaries
    Report {
        /// Steward state directory
        #[arg(long)]
        state: PathBuf,
        /// Snapshot id used to filter recorded external evidence
        #[arg(long)]
        snapshot_id: Option<String>,
    },
    /// Record constrained evidence from an external delegated tool or monitor
    Record {
        /// Steward state directory
        #[arg(long)]
        state: PathBuf,
        /// Snapshot id that the external evidence proves
        #[arg(long)]
        snapshot_id: String,
        /// Evidence item to record; repeat for multiple entries
        #[arg(long, value_parser = ["verified_offsite_copy"])]
        evidence: Vec<String>,
        /// External tool or monitor that produced the evidence
        #[arg(long)]
        source: String,
        /// Human-readable evidence detail
        #[arg(long)]
        detail: Option<String>,
        /// Private artifact reference maintained outside public Northroot
        #[arg(long)]
        artifact_ref: Option<String>,
    },
}

/// Steward retention subcommands.
#[derive(Subcommand)]
pub enum StewardRetentionCommand {
    /// Evaluate whether retention actions are allowed for a snapshot
    Evaluate {
        /// Steward state directory
        #[arg(long)]
        state: PathBuf,
        /// Snapshot id being evaluated
        #[arg(long)]
        snapshot_id: String,
        /// Evidence available for the snapshot; repeat for multiple entries
        #[arg(long)]
        evidence: Vec<String>,
        /// Include evidence derived from steward run summaries
        #[arg(long)]
        use_recorded_evidence: bool,
    },
}

/// Runs a steward custody profile subcommand by delegating to northroot-custody.
pub fn run(command: StewardCommand) -> Result<(), Box<dyn std::error::Error>> {
    let args = match command {
        StewardCommand::Init {
            inventory,
            policy,
            output,
            profile_name,
            secret_bindings,
            repository_bindings,
        } => {
            let mut args: Vec<OsString> = vec![
                "steward".into(),
                "init".into(),
                "--inventory".into(),
                inventory.into_os_string(),
                "--policy".into(),
                policy.into_os_string(),
                "--output".into(),
                output.into_os_string(),
                "--profile-name".into(),
                profile_name.into(),
            ];
            if let Some(secret_bindings) = secret_bindings {
                args.push("--secret-bindings".into());
                args.push(secret_bindings.into_os_string());
            }
            if let Some(repository_bindings) = repository_bindings {
                args.push("--repository-bindings".into());
                args.push(repository_bindings.into_os_string());
            }
            args
        }
        StewardCommand::Status { state } => vec![
            "steward".into(),
            "status".into(),
            "--state".into(),
            state.into_os_string(),
        ],
        StewardCommand::Preflight { state } => vec![
            "steward".into(),
            "preflight".into(),
            "--state".into(),
            state.into_os_string(),
        ],
        StewardCommand::VerifyState { state, snapshot_id } => {
            let mut args: Vec<OsString> = vec![
                "steward".into(),
                "verify-state".into(),
                "--state".into(),
                state.into_os_string(),
            ];
            if let Some(snapshot_id) = snapshot_id {
                args.push("--snapshot-id".into());
                args.push(snapshot_id.into());
            }
            args
        }
        StewardCommand::Capabilities { state } => vec![
            "steward".into(),
            "capabilities".into(),
            "--state".into(),
            state.into_os_string(),
        ],
        StewardCommand::CommandPlan {
            state,
            operation,
            snapshot_id,
            target,
            execute,
            scheduler,
            schedule_operation,
            every_minutes,
            runner_command,
            evidence,
            source,
            detail,
            artifact_ref,
            force,
            use_recorded_evidence,
            skip_preflight,
        } => {
            let mut args: Vec<OsString> = vec![
                "steward".into(),
                "command-plan".into(),
                "--state".into(),
                state.into_os_string(),
                "--operation".into(),
                operation.into(),
            ];
            if let Some(snapshot_id) = snapshot_id {
                args.push("--snapshot-id".into());
                args.push(snapshot_id.into());
            }
            if let Some(target) = target {
                args.push("--target".into());
                args.push(target.into_os_string());
            }
            if execute {
                args.push("--execute".into());
            }
            if let Some(scheduler) = scheduler {
                args.push("--scheduler".into());
                args.push(scheduler.into());
            }
            if let Some(schedule_operation) = schedule_operation {
                args.push("--schedule-operation".into());
                args.push(schedule_operation.into());
            }
            if let Some(every_minutes) = every_minutes {
                args.push("--every-minutes".into());
                args.push(every_minutes.to_string().into());
            }
            if let Some(runner_command) = runner_command {
                args.push("--runner-command".into());
                args.push(runner_command.into());
            }
            for evidence_item in evidence {
                args.push("--evidence".into());
                args.push(evidence_item.into());
            }
            if let Some(source) = source {
                args.push("--source".into());
                args.push(source.into());
            }
            if let Some(detail) = detail {
                args.push("--detail".into());
                args.push(detail.into());
            }
            if let Some(artifact_ref) = artifact_ref {
                args.push("--artifact-ref".into());
                args.push(artifact_ref.into());
            }
            if force {
                args.push("--force".into());
            }
            if use_recorded_evidence {
                args.push("--use-recorded-evidence".into());
            }
            if skip_preflight {
                args.push("--skip-preflight".into());
            }
            args
        }
        StewardCommand::Report { state, snapshot_id } => {
            let mut args: Vec<OsString> = vec![
                "steward".into(),
                "report".into(),
                "--state".into(),
                state.into_os_string(),
            ];
            if let Some(snapshot_id) = snapshot_id {
                args.push("--snapshot-id".into());
                args.push(snapshot_id.into());
            }
            args
        }
        StewardCommand::Evidence { command } => match command {
            StewardEvidenceCommand::Report { state, snapshot_id } => {
                let mut args: Vec<OsString> = vec![
                    "steward".into(),
                    "evidence".into(),
                    "report".into(),
                    "--state".into(),
                    state.into_os_string(),
                ];
                if let Some(snapshot_id) = snapshot_id {
                    args.push("--snapshot-id".into());
                    args.push(snapshot_id.into());
                }
                args
            }
            StewardEvidenceCommand::Record {
                state,
                snapshot_id,
                evidence,
                source,
                detail,
                artifact_ref,
            } => {
                let mut args: Vec<OsString> = vec![
                    "steward".into(),
                    "evidence".into(),
                    "record".into(),
                    "--state".into(),
                    state.into_os_string(),
                    "--snapshot-id".into(),
                    snapshot_id.into(),
                    "--source".into(),
                    source.into(),
                ];
                for evidence_item in evidence {
                    args.push("--evidence".into());
                    args.push(evidence_item.into());
                }
                if let Some(detail) = detail {
                    args.push("--detail".into());
                    args.push(detail.into());
                }
                if let Some(artifact_ref) = artifact_ref {
                    args.push("--artifact-ref".into());
                    args.push(artifact_ref.into());
                }
                args
            }
        },
        StewardCommand::Offsite { command } => match command {
            StewardOffsiteCommand::Report { state, snapshot_id } => vec![
                "steward".into(),
                "offsite".into(),
                "report".into(),
                "--state".into(),
                state.into_os_string(),
                "--snapshot-id".into(),
                snapshot_id.into(),
            ],
        },
        StewardCommand::Run {
            state,
            snapshot_id,
            execute,
        } => {
            let mut args: Vec<OsString> = vec![
                "steward".into(),
                "run".into(),
                "--state".into(),
                state.into_os_string(),
            ];
            if let Some(snapshot_id) = snapshot_id {
                args.push("--snapshot-id".into());
                args.push(snapshot_id.into());
            }
            if execute {
                args.push("--execute".into());
            }
            args
        }
        StewardCommand::Verify {
            state,
            snapshot_id,
            execute,
        } => {
            let mut args: Vec<OsString> = vec![
                "steward".into(),
                "verify".into(),
                "--state".into(),
                state.into_os_string(),
            ];
            if let Some(snapshot_id) = snapshot_id {
                args.push("--snapshot-id".into());
                args.push(snapshot_id.into());
            }
            if execute {
                args.push("--execute".into());
            }
            args
        }
        StewardCommand::Restore {
            state,
            snapshot_id,
            target,
            execute,
        } => {
            let mut args: Vec<OsString> = vec![
                "steward".into(),
                "restore".into(),
                "--state".into(),
                state.into_os_string(),
                "--snapshot-id".into(),
                snapshot_id.into(),
                "--target".into(),
                target.into_os_string(),
            ];
            if execute {
                args.push("--execute".into());
            }
            args
        }
        StewardCommand::RestoreDrill {
            state,
            target,
            snapshot_id,
            execute,
        } => {
            let mut args: Vec<OsString> = vec![
                "steward".into(),
                "restore-drill".into(),
                "--state".into(),
                state.into_os_string(),
            ];
            if let Some(target) = target {
                args.push("--target".into());
                args.push(target.into_os_string());
            }
            if let Some(snapshot_id) = snapshot_id {
                args.push("--snapshot-id".into());
                args.push(snapshot_id.into());
            }
            if execute {
                args.push("--execute".into());
            }
            args
        }
        StewardCommand::Schedule { command } => match command {
            StewardScheduleCommand::Create {
                state,
                scheduler,
                operation,
                every_minutes,
                runner_command,
            } => vec![
                "steward".into(),
                "schedule".into(),
                "create".into(),
                "--state".into(),
                state.into_os_string(),
                "--scheduler".into(),
                scheduler.into(),
                "--operation".into(),
                operation.into(),
                "--every-minutes".into(),
                every_minutes.to_string().into(),
                "--runner-command".into(),
                runner_command.into(),
            ],
            StewardScheduleCommand::Status { state } => vec![
                "steward".into(),
                "schedule".into(),
                "status".into(),
                "--state".into(),
                state.into_os_string(),
            ],
            StewardScheduleCommand::Install {
                state,
                execute,
                skip_preflight,
            } => {
                let mut args: Vec<OsString> = vec![
                    "steward".into(),
                    "schedule".into(),
                    "install".into(),
                    "--state".into(),
                    state.into_os_string(),
                ];
                if execute {
                    args.push("--execute".into());
                }
                if skip_preflight {
                    args.push("--skip-preflight".into());
                }
                args
            }
            StewardScheduleCommand::Uninstall { state, execute } => {
                let mut args: Vec<OsString> = vec![
                    "steward".into(),
                    "schedule".into(),
                    "uninstall".into(),
                    "--state".into(),
                    state.into_os_string(),
                ];
                if execute {
                    args.push("--execute".into());
                }
                args
            }
            StewardScheduleCommand::Delete { state, force } => {
                let mut args: Vec<OsString> = vec![
                    "steward".into(),
                    "schedule".into(),
                    "delete".into(),
                    "--state".into(),
                    state.into_os_string(),
                ];
                if force {
                    args.push("--force".into());
                }
                args
            }
        },
        StewardCommand::Retention { command } => match command {
            StewardRetentionCommand::Evaluate {
                state,
                snapshot_id,
                evidence,
                use_recorded_evidence,
            } => {
                let mut args: Vec<OsString> = vec![
                    "steward".into(),
                    "retention".into(),
                    "evaluate".into(),
                    "--state".into(),
                    state.into_os_string(),
                    "--snapshot-id".into(),
                    snapshot_id.into(),
                ];
                for evidence_item in evidence {
                    args.push("--evidence".into());
                    args.push(evidence_item.into());
                }
                if use_recorded_evidence {
                    args.push("--use-recorded-evidence".into());
                }
                args
            }
        },
    };
    run_custody_cli(args)
}

fn run_custody_cli(args: Vec<OsString>) -> Result<(), Box<dyn std::error::Error>> {
    let mut command = Command::new(python_executable());
    command
        .arg("-m")
        .arg("northroot.custody.cli")
        .args(args)
        .env("PYTHONPATH", custody_pythonpath());
    let status = command.status().map_err(|err| {
        format!(
            "failed to launch northroot-custody via python; install northroot-custody or set NORTHROOT_CUSTODY_PYTHONPATH: {err}"
        )
    })?;
    if !status.success() {
        return Err(format!("northroot-custody steward command failed with {status}").into());
    }
    Ok(())
}

fn python_executable() -> OsString {
    std::env::var_os("NORTHROOT_PYTHON").unwrap_or_else(|| OsString::from("python3"))
}

fn custody_pythonpath() -> OsString {
    let package_path = std::env::var_os("NORTHROOT_CUSTODY_PYTHONPATH")
        .or_else(|| source_tree_custody_path().map(OsString::from));
    match (package_path, std::env::var_os("PYTHONPATH")) {
        (Some(package_path), Some(existing)) if !existing.is_empty() => {
            let mut combined = package_path;
            combined.push(if cfg!(windows) { ";" } else { ":" });
            combined.push(existing);
            combined
        }
        (Some(package_path), _) => package_path,
        (None, Some(existing)) => existing,
        (None, None) => OsString::new(),
    }
}

fn source_tree_custody_path() -> Option<PathBuf> {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let candidate = manifest_dir
        .parent()?
        .parent()?
        .join("packages")
        .join("northroot-custody")
        .join("src");
    if candidate.is_dir() {
        Some(candidate)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsString;
    use std::fs;
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;
    use tempfile::TempDir;

    struct EnvGuard {
        key: &'static str,
        old_value: Option<OsString>,
    }

    impl EnvGuard {
        fn set(key: &'static str, value: OsString) -> Self {
            let old_value = std::env::var_os(key);
            std::env::set_var(key, value);
            Self { key, old_value }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            if let Some(old_value) = self.old_value.take() {
                std::env::set_var(self.key, old_value);
            } else {
                std::env::remove_var(self.key);
            }
        }
    }

    fn examples_dir() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("packages")
            .join("northroot-custody")
            .join("examples")
    }

    fn write_fake_executable(dir: &Path, name: &str) -> PathBuf {
        let path = dir.join(name);
        fs::write(&path, "#!/bin/sh\nexit 0\n").unwrap();
        #[cfg(unix)]
        {
            let mut permissions = fs::metadata(&path).unwrap().permissions();
            permissions.set_mode(0o755);
            fs::set_permissions(&path, permissions).unwrap();
        }
        path
    }

    #[test]
    fn steward_init_status_run_and_schedule_delegate_to_custody_package() {
        let temp = TempDir::new().unwrap();
        let state = temp.path().join("steward");
        let examples = examples_dir();

        run(StewardCommand::Init {
            inventory: examples.join("workspace-inventory.example.json"),
            policy: examples.join("custody-policy.example.json"),
            output: state.clone(),
            profile_name: "steward".to_string(),
            secret_bindings: Some(examples.join("secret-bindings.redacted.example.json")),
            repository_bindings: Some(examples.join("repository-bindings.redacted.example.json")),
        })
        .unwrap();
        assert!(state.join("snapshot-plan.json").is_file());
        assert!(state.join("resticprofile.yaml").is_file());
        assert!(state.join("steward-installation.json").is_file());
        let resticprofile_config =
            std::fs::read_to_string(state.join("resticprofile.yaml")).unwrap();
        assert!(resticprofile_config.contains("password-command"));

        run(StewardCommand::Status {
            state: state.clone(),
        })
        .unwrap();
        run(StewardCommand::Capabilities {
            state: state.clone(),
        })
        .unwrap();
        run(StewardCommand::CommandPlan {
            state: state.clone(),
            operation: "report".to_string(),
            snapshot_id: Some("snap-001".to_string()),
            target: None,
            execute: false,
            scheduler: None,
            schedule_operation: None,
            every_minutes: None,
            runner_command: None,
            evidence: vec![],
            source: None,
            detail: None,
            artifact_ref: None,
            force: false,
            use_recorded_evidence: false,
            skip_preflight: false,
        })
        .unwrap();
        run(StewardCommand::Report {
            state: state.clone(),
            snapshot_id: Some("snap-001".to_string()),
        })
        .unwrap();
        run(StewardCommand::Retention {
            command: StewardRetentionCommand::Evaluate {
                state: state.clone(),
                snapshot_id: "snap-001".to_string(),
                evidence: vec![
                    "verified_snapshot".to_string(),
                    "verified_offsite_copy".to_string(),
                    "restore_drill".to_string(),
                ],
                use_recorded_evidence: false,
            },
        })
        .unwrap();
        let fake_bin = temp.path().join("bin");
        fs::create_dir(&fake_bin).unwrap();
        write_fake_executable(&fake_bin, "resticprofile");
        write_fake_executable(&fake_bin, "op");
        let existing_path = std::env::var_os("PATH").unwrap_or_default();
        let mut path_with_fake_tools = OsString::from(fake_bin);
        path_with_fake_tools.push(if cfg!(windows) { ";" } else { ":" });
        path_with_fake_tools.push(existing_path);
        let _path_guard = EnvGuard::set("PATH", path_with_fake_tools);
        let _token_guard = EnvGuard::set("OP_SERVICE_ACCOUNT_TOKEN", OsString::from("dummy"));
        run(StewardCommand::Preflight {
            state: state.clone(),
        })
        .unwrap();
        run(StewardCommand::VerifyState {
            state: state.clone(),
            snapshot_id: None,
        })
        .unwrap();
        run(StewardCommand::Run {
            state: state.clone(),
            snapshot_id: None,
            execute: false,
        })
        .unwrap();
        run(StewardCommand::Verify {
            state: state.clone(),
            snapshot_id: Some("snap-001".to_string()),
            execute: false,
        })
        .unwrap();
        run(StewardCommand::Restore {
            state: state.clone(),
            snapshot_id: "snap-001".to_string(),
            target: temp.path().join("recovery-restore"),
            execute: false,
        })
        .unwrap();
        run(StewardCommand::RestoreDrill {
            state: state.clone(),
            target: None,
            snapshot_id: Some("snap-001".to_string()),
            execute: false,
        })
        .unwrap();
        run(StewardCommand::Evidence {
            command: StewardEvidenceCommand::Record {
                state: state.clone(),
                snapshot_id: "snap-001".to_string(),
                evidence: vec!["verified_offsite_copy".to_string()],
                source: "external-monitor://offsite-copy-check".to_string(),
                detail: Some("offsite repository check passed".to_string()),
                artifact_ref: Some("artifact://private/offsite-check/run-001".to_string()),
            },
        })
        .unwrap();
        run(StewardCommand::Offsite {
            command: StewardOffsiteCommand::Report {
                state: state.clone(),
                snapshot_id: "snap-001".to_string(),
            },
        })
        .unwrap();
        run(StewardCommand::Evidence {
            command: StewardEvidenceCommand::Report {
                state: state.clone(),
                snapshot_id: Some("snap-001".to_string()),
            },
        })
        .unwrap();
        let summaries_dir = state.join("run-summaries");
        assert!(summaries_dir.is_dir());
        let summary_count = std::fs::read_dir(&summaries_dir).unwrap().count();
        assert!(summary_count >= 3);
        run(StewardCommand::Schedule {
            command: StewardScheduleCommand::Create {
                state: state.clone(),
                scheduler: "launchd".to_string(),
                operation: "verify".to_string(),
                every_minutes: 60,
                runner_command: "nr steward".to_string(),
            },
        })
        .unwrap();
        assert!(state
            .join("schedules")
            .join("org.northroot.steward.plist")
            .is_file());
        let launchd_template =
            std::fs::read_to_string(state.join("schedules").join("org.northroot.steward.plist"))
                .unwrap();
        assert!(launchd_template.contains("nr steward verify --state"));
        run(StewardCommand::Schedule {
            command: StewardScheduleCommand::Status {
                state: state.clone(),
            },
        })
        .unwrap();
        run(StewardCommand::Schedule {
            command: StewardScheduleCommand::Install {
                state: state.clone(),
                execute: false,
                skip_preflight: false,
            },
        })
        .unwrap();
        run(StewardCommand::Schedule {
            command: StewardScheduleCommand::Uninstall {
                state: state.clone(),
                execute: false,
            },
        })
        .unwrap();
        run(StewardCommand::Schedule {
            command: StewardScheduleCommand::Delete {
                state: state.clone(),
                force: false,
            },
        })
        .unwrap();
        assert!(!state
            .join("schedules")
            .join("org.northroot.steward.plist")
            .is_file());
    }
}
