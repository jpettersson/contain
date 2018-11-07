use std::process::Command;
use std::fs::{canonicalize};
// use std::path::PathBuf;

static WITHOUT_ARGS_OUTPUT: &'static str = "contain 0.1.0
Jonathan Pettersson
Runs your development tool inside a container

USAGE:
    contain <command> [args]...

FLAGS:
    -h, --help    Prints help information

ARGS:
    <command>    the command you want to run inside a container
    <args>...    
";
 
static LS_IN_EXAMPLES_MULTIPLE_CONTAINERS: &'static str = "
Dockerfile.mvn
Dockerfile.yarn
";

static ERROR_NO_CONFIG_FILE_FOUND: &'static str = "Error: \u{1b}[31mNo docker image found for 'ls' in .contain.yaml or any path above!\u{1b}[0m
";

#[cfg(test)]
mod integration {
    use Command;
    use canonicalize;
    use WITHOUT_ARGS_OUTPUT;
    use LS_IN_EXAMPLES_MULTIPLE_CONTAINERS;
    use ERROR_NO_CONFIG_FILE_FOUND;

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
    
        assert_eq!(String::from_utf8_lossy(&output.stderr), WITHOUT_ARGS_OUTPUT);
    }

    // Test if it's possible to execute a simple command through a docker container

    #[test]
    fn calling_command_through_docker_works() {
        let output = Command::new(canonicalize("./target/debug/contain").unwrap())
            .arg("ls")
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
        let output = Command::new(canonicalize("./target/debug/contain").unwrap())
            .arg("ls") // Will run in current project root which does not have a .contain.yaml file
            .output()
            .expect("failed to execute process");

        assert_eq!(String::from_utf8_lossy(&output.stderr), ERROR_NO_CONFIG_FILE_FOUND);
    }
}

