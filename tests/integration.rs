use std::process::Command;
use std::fs::{canonicalize};
use std::path::PathBuf;

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
 
static LS_IN_EXAMPLES_MULTIPLE_CONTAINERS: &'static str = "docker [\"run\", \"-u\", \"501:20\", \"--rm\", \"--mount\", \"type=bind,src=/Users/jpettersson/code/github.com/jpettersson/contain/examples/multiple-containers,dst=/workdir\", \"gcr.io/styra-infra/yarn:latest\", \"ls\"]
Dockerfile.mvn
Dockerfile.yarn
node_modules
yarn.lock
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

    #[test]
    fn calling_command_through_docker_works() {
        let output = Command::new(canonicalize("./target/debug/contain").unwrap())
            .arg("ls")
            .current_dir(canonicalize("examples/multiple-containers").unwrap())
            .output()
            .expect("failed to execute process");

        assert_eq!(String::from_utf8_lossy(&output.stdout), LS_IN_EXAMPLES_MULTIPLE_CONTAINERS);
    }
    
    #[test]
    fn calling_command_in_path_without_config_yields_error() {
        let output = Command::new(canonicalize("./target/debug/contain").unwrap())
            .arg("ls")
            .current_dir(canonicalize("./tests/fixtures/missing_contain_yaml").unwrap())
            .output()
            .expect("failed to execute process");

        assert_eq!(String::from_utf8_lossy(&output.stderr), ERROR_NO_CONFIG_FILE_FOUND);
    }
}

