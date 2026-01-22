use std::process::Command;
use std::fs::canonicalize;
use std::path::Path;

// Check for key elements in help output rather than exact match
static HELP_SHOULD_CONTAIN: &[&str] = &[
    "contain",
    "Jonathan Pettersson",
    "Runs your development tools inside containers",
    "USAGE:",
    "SUBCOMMANDS:",
    "run",
    "shell",
    "up",
    "down",
    "status",
];
 
static LS_IN_EXAMPLES_MULTIPLE_CONTAINERS: &'static str = "
Dockerfile.mvn
Dockerfile.yarn
";

static ERROR_NO_CONFIG_FILE_FOUND: &'static str = "No docker image found for 'ls' in .contain.yaml or any path above
";

#[cfg(test)]
mod integration {
    use super::*;

    pub trait ReversableSubString { 
        fn take_from_end(self, len: usize) -> Self;
    }

    impl ReversableSubString for String {
        fn take_from_end(self, len: usize) -> String {
            let output_sub_rev : String = self
                                        .chars()
                                        .rev()
                                        .take(len)
                                        .collect();

            let output_sub : String = output_sub_rev
                                        .chars()
                                        .rev()
                                        .collect();

            return output_sub;
        }
    }

    #[test]
    #[ignore]  // Requires Docker in PATH - run with `cargo test -- --ignored`
    fn docker_is_available() {
        let status = Command::new("docker")
            .arg("-v")
            .status()
            .expect("failed to execute process");

        assert_eq!(status.success(), true);
    }

    #[test]
    fn calling_contain_without_args() {
        let output = Command::new(canonicalize("./target/debug/contain").unwrap())
            .output()
            .expect("failed to execute process");

        let stderr = String::from_utf8_lossy(&output.stderr);
        for expected in HELP_SHOULD_CONTAIN {
            assert!(stderr.contains(expected), "Help output should contain '{}'. Got: {}", expected, stderr);
        }
    }

    // Test if it's possible to execute a simple command through a docker container

    #[test]
    #[ignore]  // Requires Docker in PATH - run with `cargo test -- --ignored`
    fn calling_command_through_docker_works() {
        let output = Command::new(canonicalize("./target/debug/contain").unwrap())
            .args(&["run", "ls"])
            .env("CONTAIN_PASSTHROUGH", "0")  // Disable passthrough for testing
            .current_dir(canonicalize("examples/multiple-containers").unwrap())
            .output()
            .expect("failed to execute process");

        let output_str = String::from_utf8_lossy(&output.stdout).to_string();

        // Only compare the last part of the output as there are variables that changes between systems in the CLI output
        let output_sub = output_str.take_from_end(LS_IN_EXAMPLES_MULTIPLE_CONTAINERS.len());

        assert_eq!(output_sub, LS_IN_EXAMPLES_MULTIPLE_CONTAINERS);
    }
    
    #[test]
    fn calling_command_in_path_without_config_yields_error() {
        // Run from temp directory which has no .contain.yaml in its parent chain
        let output = Command::new(canonicalize("./target/debug/contain").unwrap())
            .args(&["run", "ls"])
            .env("CONTAIN_PASSTHROUGH", "0")  // Disable passthrough for testing
            .current_dir(std::env::temp_dir())
            .output()
            .expect("failed to execute process");

        assert_eq!(String::from_utf8_lossy(&output.stderr), ERROR_NO_CONFIG_FILE_FOUND);
    }
}

/// Tests that verify Docker command generation without requiring Docker.
/// These tests use the `--dry` flag to capture the generated command.
#[cfg(test)]
mod dry_run_tests {
    use super::*;

    /// Helper to run contain with --dry flag and capture output
    fn run_dry(dir: &Path, args: &[&str]) -> (String, String, bool) {
        let output = Command::new(canonicalize("./target/debug/contain").unwrap())
            .current_dir(dir)
            .env("CONTAIN_PASSTHROUGH", "0")  // Disable passthrough for testing
            .args(args)
            .output()
            .expect("failed to execute contain");

        (
            String::from_utf8_lossy(&output.stdout).to_string(),
            String::from_utf8_lossy(&output.stderr).to_string(),
            output.status.success(),
        )
    }

    #[test]
    fn dry_run_includes_basic_docker_flags() {
        let (stdout, _, success) = run_dry(
            Path::new("tests/fixtures/basic"),
            &["--dry", "run", "echo", "hello"],
        );

        assert!(success, "Command should succeed");
        assert!(stdout.contains("docker"), "Output should contain 'docker'");
        assert!(stdout.contains("run"), "Output should contain 'run'");
        assert!(stdout.contains("--rm"), "Output should contain '--rm' flag");
        assert!(stdout.contains("-w"), "Output should contain '-w' working dir flag");
        assert!(stdout.contains("--mount"), "Output should contain '--mount' flag");
        assert!(stdout.contains("test-image:latest"), "Output should contain image name");
        assert!(stdout.contains("echo"), "Output should contain the command");
        assert!(stdout.contains("hello"), "Output should contain the command args");
    }

    #[test]
    fn dry_run_includes_user_flag() {
        let (stdout, _, success) = run_dry(
            Path::new("tests/fixtures/basic"),
            &["--dry", "run", "echo", "hello"],
        );

        assert!(success);
        // Should include -u flag with uid:gid format
        assert!(stdout.contains("-u "), "Output should contain '-u' user flag");
    }

    #[test]
    fn dry_run_interactive_adds_it_flags() {
        let (stdout, _, success) = run_dry(
            Path::new("tests/fixtures/basic"),
            &["--dry", "run", "-i", "echo", "hello"],
        );

        assert!(success);
        assert!(stdout.contains("-it"), "Output should contain '-it' flags for interactive mode");
    }

    #[test]
    fn dry_run_keep_container_skips_rm() {
        let (stdout, _, success) = run_dry(
            Path::new("tests/fixtures/basic"),
            &["--dry", "-k", "run", "echo", "hello"],
        );

        assert!(success);
        assert!(!stdout.contains("--rm"), "Output should NOT contain '--rm' when -k flag is used");
    }

    #[test]
    fn dry_run_root_skips_user_flag() {
        let (stdout, _, success) = run_dry(
            Path::new("tests/fixtures/basic"),
            &["--dry", "--root", "run", "echo", "hello"],
        );

        assert!(success);
        // Should NOT include -u flag when running as root
        assert!(!stdout.contains("-u "), "Output should NOT contain '-u' flag when --root is used");
    }

    #[test]
    fn dry_run_env_variables_appear_as_e_flags() {
        let (stdout, _, success) = run_dry(
            Path::new("tests/fixtures/with-env"),
            &["--dry", "run", "echo", "hello"],
        );

        assert!(success);
        assert!(stdout.contains("-e"), "Output should contain '-e' flag for env variables");
        assert!(stdout.contains("MY_VAR=test_value"), "Output should contain first env variable");
        assert!(stdout.contains("ANOTHER_VAR=another_value"), "Output should contain second env variable");
    }

    #[test]
    fn dry_run_ports_appear_as_p_flags() {
        let (stdout, _, success) = run_dry(
            Path::new("tests/fixtures/with-ports"),
            &["--dry", "run", "echo", "hello"],
        );

        assert!(success);
        assert!(stdout.contains("-p"), "Output should contain '-p' flag for ports");
        assert!(stdout.contains("8080:80"), "Output should contain first port mapping");
        assert!(stdout.contains("3000:3000"), "Output should contain second port mapping");
    }

    #[test]
    fn dry_run_skip_ports_omits_port_mappings() {
        let (stdout, _, success) = run_dry(
            Path::new("tests/fixtures/with-ports"),
            &["--dry", "--skip-ports", "run", "echo", "hello"],
        );

        assert!(success);
        assert!(!stdout.contains("-p "), "Output should NOT contain '-p' flag when --skip-ports is used");
        assert!(!stdout.contains("8080:80"), "Output should NOT contain port mappings when --skip-ports is used");
    }

    #[test]
    fn dry_run_mounts_appear_as_mount_flags() {
        let (stdout, _, success) = run_dry(
            Path::new("tests/fixtures/with-mounts"),
            &["--dry", "run", "echo", "hello"],
        );

        assert!(success);
        // Should have at least 2 --mount flags (workspace + custom)
        let mount_count = stdout.matches("--mount").count();
        assert!(mount_count >= 2, "Output should contain at least 2 --mount flags (workspace + custom), found {}", mount_count);
        assert!(stdout.contains("/tmp"), "Output should contain custom mount source path");
        assert!(stdout.contains("/container-tmp"), "Output should contain custom mount destination path");
    }

    #[test]
    fn no_config_file_shows_error() {
        let temp_dir = std::env::temp_dir();
        let (_, stderr, success) = run_dry(&temp_dir, &["--dry", "run", "echo", "hello"]);

        assert!(!success, "Command should fail without config file");
        assert!(
            stderr.contains("No docker image found for 'echo'"),
            "Error message should indicate no config found. Got: {}",
            stderr
        );
    }

    #[test]
    fn dry_run_with_multiple_args() {
        let (stdout, _, success) = run_dry(
            Path::new("tests/fixtures/basic"),
            &["--dry", "run", "ls", "-la", "/tmp"],
        );

        assert!(success);
        assert!(stdout.contains("ls"), "Output should contain the command");
        assert!(stdout.contains("-la"), "Output should contain first arg");
        assert!(stdout.contains("/tmp"), "Output should contain second arg");
    }

    #[test]
    fn dry_run_combined_flags() {
        let (stdout, _, success) = run_dry(
            Path::new("tests/fixtures/basic"),
            &["--dry", "-k", "run", "-i", "echo", "test"],
        );

        assert!(success);
        assert!(stdout.contains("-it"), "Output should contain '-it' for interactive");
        assert!(!stdout.contains("--rm"), "Output should NOT contain '--rm' when -k is used");
    }
}

