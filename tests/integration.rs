use std::process::Command;

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
 
#[cfg(test)]
mod integration {
    use Command;
    use WITHOUT_ARGS_OUTPUT;

    #[test]
    fn calling_contain_without_args() {
        let output = Command::new("./target/debug/contain")
            .output()
            .expect("failed to execute process");
    
        assert_eq!(String::from_utf8_lossy(&output.stderr), WITHOUT_ARGS_OUTPUT);
    }
    
}