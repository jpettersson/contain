use std::process::{Command, Stdio, exit};
use std::path::PathBuf;
use std::collections::HashMap;
use std::env;

use clap::{Arg, App, AppSettings};
use colored::*;
use quick_error::quick_error;
use users::{get_user_by_uid, get_current_uid, get_current_gid};
use semver::Version;

quick_error! {
    #[derive(Debug)]
    pub enum Error {
        DockerError(descr: String) {
            display("Docker error: {}", descr)
        }
        ConfigError(descr: String) {
            display("Configuration error: {}", descr)
        }
        ConfigMissingField { file: String, field: String } {
            display("Missing required field '{}' in {}", field, file)
        }
        ConfigInvalidValue { file: String, field: String, reason: String } {
            display("Invalid value for '{}' in {}: {}", field, file, reason)
        }
        PathError(descr: String) {
            display("Path error: {}", descr)
        }
        CommandError { cmd: String, reason: String } {
            display("Failed to execute '{}': {}", cmd, reason)
        }
        UnsupportedParameters(descr: String) {
            display("Unsupported parameter: {}", descr)
        }
        NoConfigFound { command: String } {
            display("No docker image found for '{}' in .contain.yaml or any path above", command)
        }
        ImageBuildFailed { image: String, dockerfile: String } {
            display("Unable to build docker image '{}' from dockerfile '{}'", image, dockerfile)
        }
    }
}

const COMMAND: &str = "command";
const ARGS: &str = "args";
const CONTAIN_FILENAME: &str = ".contain.yaml";

#[derive(Debug)]
struct GlobalOptions {
    interactive: bool,
    persist_image: bool,
    keep_container: bool,
    run_as_root: bool,
    dry_run: bool,
    skip_ports: bool,
    skip_name: bool,
    cli_env_variables: Vec<String>
}

impl GlobalOptions {
    fn persist_image(&mut self, a: bool) {
        self.persist_image = a;
    }

    fn keep_container(&mut self, a: bool) {
        self.keep_container = a;
    }

    fn dry_run(&mut self, a: bool) {
        self.dry_run = a;
    }

    fn interactive(&mut self, a: bool) {
        self.interactive = a;
    }

    fn run_as_root(&mut self, a: bool) {
        self.run_as_root = a;
    }

    fn skip_ports(&mut self, a: bool) {
        self.skip_ports = a;
    }

    fn skip_name(&mut self, a: bool) {
        self.skip_name = a;
    }

    fn add_env_variable(&mut self, a: String) {
        self.cli_env_variables.push(a);
    }
}

/// Detects if contain is running inside a container.
///
/// Detection priority:
/// 1. CONTAIN_PASSTHROUGH env var (explicit override)
/// 2. /.dockerenv file (Docker)
/// 3. /run/.containerenv file (Podman)
fn is_inside_container() -> bool {
    // Allow explicit override via environment variable
    if let Ok(val) = std::env::var("CONTAIN_PASSTHROUGH") {
        match val.to_lowercase().as_str() {
            "1" | "true" | "yes" => return true,
            "0" | "false" | "no" => return false,
            _ => {} // Fall through to auto-detection
        }
    }

    // Check for Docker container marker
    if std::path::Path::new("/.dockerenv").exists() {
        return true;
    }

    // Check for Podman container marker
    if std::path::Path::new("/run/.containerenv").exists() {
        return true;
    }

    false
}

/// Execute command directly in passthrough mode (when inside a container).
/// Preserves -e environment variables, strips all other contain flags.
#[cfg(unix)]
fn passthrough_command(command: &str, args: Vec<&str>, options: &GlobalOptions) -> ! {
    use std::os::unix::process::CommandExt;

    // Set environment variables from -e flags
    for env_var in &options.cli_env_variables {
        if let Some(pos) = env_var.find('=') {
            let key = &env_var[..pos];
            let value = &env_var[pos + 1..];
            // SAFETY: This is single-threaded CLI startup code
            unsafe { std::env::set_var(key, value); }
        }
    }

    // Optional: show passthrough indicator (controlled by env var)
    if std::env::var("CONTAIN_VERBOSE").is_ok() {
        eprintln!("{} passthrough: {} {}", "(contain)".blue().bold(), command, args.join(" "));
    }

    // Build the command and use exec to replace the current process
    let err = Command::new(command)
        .args(&args)
        .exec();

    // exec() only returns if there was an error
    eprintln!("contain: failed to execute '{}': {}", command, err);
    std::process::exit(127)
}

#[derive(Debug)]
struct Configuration {
    image: String,
    name: Option<String>,
    dockerfile: String,
    root_path: PathBuf,
    flags: Vec<String>,
    workdir_path: String,
    env_variables: Vec<String>,
    build_args: Vec<String>,
    extra_mounts: Vec<String>,
    ports: Vec<String>
}

fn get_required_string(table: &HashMap<String, config::Value>, field: &str, file: &str) -> Result<String, Error> {
    table.get(field)
        .ok_or_else(|| Error::ConfigMissingField {
            file: file.to_string(),
            field: field.to_string()
        })?
        .clone()
        .into_string()
        .map_err(|_| Error::ConfigInvalidValue {
            file: file.to_string(),
            field: field.to_string(),
            reason: "expected a string".to_string()
        })
}

fn get_optional_string(table: &HashMap<String, config::Value>, field: &str, file: &str) -> Result<Option<String>, Error> {
    match table.get(field) {
        None => Ok(None),
        Some(v) => v.clone()
            .into_string()
            .map(Some)
            .map_err(|_| Error::ConfigInvalidValue {
                file: file.to_string(),
                field: field.to_string(),
                reason: "expected a string".to_string()
            })
    }
}

fn get_string_array(table: &HashMap<String, config::Value>, field: &str, file: &str) -> Result<Vec<String>, Error> {
    match table.get(field) {
        None => Ok(Vec::new()),
        Some(node) => {
            let vec = node.clone()
                .into_array()
                .map_err(|_| Error::ConfigInvalidValue {
                    file: file.to_string(),
                    field: field.to_string(),
                    reason: "expected an array".to_string()
                })?;
            vec.into_iter()
                .map(|value: config::Value| {
                    let s = value.into_string().map_err(|_| Error::ConfigInvalidValue {
                        file: file.to_string(),
                        field: field.to_string(),
                        reason: "expected array of strings".to_string()
                    })?;
                    shellexpand::env(&s)
                        .map(|expanded| expanded.into_owned())
                        .map_err(|e| Error::ConfigInvalidValue {
                            file: file.to_string(),
                            field: field.to_string(),
                            reason: format!("environment variable expansion failed: {}", e)
                        })
                })
                .collect()
        }
    }
}

fn main() {
    if let Err(err) = run() {
        eprintln!("{}", err);
        std::process::exit(1);
    }
}

fn run() -> Result<bool, Error> {

    let mut options = GlobalOptions {
        interactive: false,
        persist_image: false,
        keep_container: false,
        dry_run: false,
        run_as_root: false,
        skip_ports: false,
        skip_name: false,
        cli_env_variables: vec![]
    };

    let matches = App::new("contain")
        .setting(AppSettings::TrailingVarArg)
        .setting(AppSettings::AllowLeadingHyphen)
        .setting(AppSettings::ArgRequiredElseHelp)
        .setting(AppSettings::DisableVersion)
        .version(env!("CARGO_PKG_VERSION"))
        .author("Jonathan Pettersson")
        .about("Runs your development tools inside containers")
            .arg(Arg::with_name(COMMAND)
                .help("the command you want to run inside a container")
                .takes_value(true)
                .required(true))
            .arg(Arg::with_name("args")
                 .multiple(true))
            .get_matches();

    if matches.is_present(COMMAND) {
        let command = matches.value_of(COMMAND).unwrap();
        if matches.is_present(ARGS) {
            let args: Vec<&str> = matches.values_of(ARGS).unwrap().collect();
            let mut num_program_flags = 0;

            let mut flag = command;
            while flag.as_bytes()[0] == b'-' {
                match flag {
                    "-p" => options.persist_image(true),
                    "-k" => options.keep_container(true),
                    "-i" => options.interactive(true),
                    "--dry" => options.dry_run(true),
                    "--root" => options.run_as_root(true),
                    "--skip-ports" => options.skip_ports(true),
                    "--skip-name" => options.skip_name(true),
                    x if x.as_bytes()[1] == b'e' => {
                        let slice = &x[2..];
                        options.add_env_variable(slice.to_string())
                        },
                    _ => return Err(Error::UnsupportedParameters(format!("{}", flag)))
                }
                num_program_flags += 1;
                flag = args[num_program_flags-1];
            }

            // Determine actual command and args after flag parsing
            let (actual_command, actual_args): (&str, Vec<&str>) = if num_program_flags > 0 {
                let mut mut_args = args.clone();
                (args[num_program_flags-1], mut_args.drain(num_program_flags..).collect())
            } else {
                (command, args)
            };

            // Check for passthrough mode (running inside a container)
            if is_inside_container() {
                passthrough_command(actual_command, actual_args, &options);
            }

            return run_command(actual_command, actual_args, options);

        }else{
            // No args case - check passthrough mode
            if is_inside_container() {
                passthrough_command(command, vec![], &options);
            }
            return run_command(command, vec![], options);
        }
    }else{
        // This always happens because clap-rs triggers help if no command is passed..
        // TODO: Get rid of this else branch.

        return Ok(true);
    }
}

fn get_config_table(config: &config::Config, command: &str) -> Option<HashMap<String, config::Value>> {
    let array = config.get_array("images").ok()?;

    for node in &array {
        let table = match node.clone().into_table() {
            Ok(t) => t,
            Err(_) => continue,
        };

        let commands_value = match table.get("commands") {
            Some(v) => v.clone(),
            None => continue,
        };

        // Check if commands is a single string
        if let Ok(string) = commands_value.clone().into_string() {
            if string == command || string == "any" {
                return Some(table.clone())
            }
        // Check if commands is an array of strings
        } else if let Ok(entries) = commands_value.into_array() {
            for entry in &entries {
                if let Ok(entry_string) = entry.clone().into_string() {
                    if entry_string == command || entry_string == "any" {
                        return Some(table.clone())
                    }
                }
            }
        }
    }

    // No matching command was found in this YAML document
    None
}

fn load_config(mut path: PathBuf, command: &str) -> Result<Configuration, Error> {

    let path_str = path.as_path()
        .to_str()
        .ok_or_else(|| Error::PathError("Path contains invalid UTF-8".to_string()))?;

    // SAFETY: This is single-threaded CLI startup code
    unsafe { env::set_var("CONTAIN_ROOT_PATH", path_str); }

    let full_path = format!("{}/{}", path_str, CONTAIN_FILENAME);

    let result = config::Config::builder()
        .add_source(config::File::with_name(&full_path))
        .build();

    if let Ok(ref config) = result {

        let min_version: Option<String> = config.get("contain_min_version").ok();

        if let Some(v) = min_version {
            if Version::parse(env!("CARGO_PKG_VERSION")) < Version::parse(&v) {
                return Err(Error::ConfigError(format!(
                    "{} requires contain version >= {} (current version: {})",
                    full_path, v, env!("CARGO_PKG_VERSION")
                )));
            }
        };

        if let Some(command_entry) = get_config_table(config, command) {

            let image = get_required_string(&command_entry, "image", &full_path)?;
            let name = get_optional_string(&command_entry, "name", &full_path)?;
            let dockerfile = get_required_string(&command_entry, "dockerfile", &full_path)?;

            // Process var definitions (execute commands to set environment variables)
            if let Some(node) = command_entry.get("var") {
                if let Ok(vec) = node.clone().into_array() {
                    for item in &vec {
                        if let Ok(obj) = item.clone().into_table() {
                            let var_name = obj.get("name")
                                .ok_or_else(|| Error::ConfigMissingField {
                                    file: full_path.clone(),
                                    field: "var[].name".to_string()
                                })?;
                            let var_cmd = obj.get("command")
                                .ok_or_else(|| Error::ConfigMissingField {
                                    file: full_path.clone(),
                                    field: "var[].command".to_string()
                                })?;

                            let var_name_string = var_name.to_string();
                            let var_cmd_string = shellexpand::env(&var_cmd.to_string())
                                .map_err(|e| Error::ConfigInvalidValue {
                                    file: full_path.clone(),
                                    field: "var[].command".to_string(),
                                    reason: format!("environment variable expansion failed: {}", e)
                                })?
                                .into_owned();

                            let result = Command::new("sh")
                                .arg("-c")
                                .arg(&var_cmd_string)
                                .output()
                                .map_err(|e| Error::CommandError {
                                    cmd: format!("sh -c '{}'", var_cmd_string),
                                    reason: e.to_string()
                                })?;

                            let output = String::from_utf8_lossy(&result.stdout)
                                .to_string()
                                .trim()
                                .to_string();

                            // SAFETY: This is single-threaded CLI startup code
                            unsafe { env::set_var(var_name_string, output); }
                        }
                    }
                }
            }

            let env_variables = get_string_array(&command_entry, "env", &full_path)?;
            let build_args = get_string_array(&command_entry, "build_args", &full_path)?;

            // Process mounts
            let mut extra_mounts: Vec<String> = Vec::new();
            if let Some(node) = command_entry.get("mounts") {
                if let Ok(vec) = node.clone().into_array() {
                    for (i, item) in vec.iter().enumerate() {
                        if let Ok(obj) = item.clone().into_table() {
                            let mount_type = obj.get("type")
                                .ok_or_else(|| Error::ConfigMissingField {
                                    file: full_path.clone(),
                                    field: format!("mounts[{}].type", i)
                                })?;
                            let src = obj.get("src")
                                .ok_or_else(|| Error::ConfigMissingField {
                                    file: full_path.clone(),
                                    field: format!("mounts[{}].src", i)
                                })?;
                            let dst = obj.get("dst")
                                .ok_or_else(|| Error::ConfigMissingField {
                                    file: full_path.clone(),
                                    field: format!("mounts[{}].dst", i)
                                })?;

                            let src_string = src.to_string();
                            let dst_string = dst.to_string();

                            let src_expanded = shellexpand::env(&src_string)
                                .map_err(|e| Error::ConfigInvalidValue {
                                    file: full_path.clone(),
                                    field: format!("mounts[{}].src", i),
                                    reason: format!("environment variable expansion failed: {}", e)
                                })?;
                            let dst_expanded = shellexpand::env(&dst_string)
                                .map_err(|e| Error::ConfigInvalidValue {
                                    file: full_path.clone(),
                                    field: format!("mounts[{}].dst", i),
                                    reason: format!("environment variable expansion failed: {}", e)
                                })?;

                            let extra_options = match obj.get("options") {
                                Some(s) => format!(",{}", s.to_string()),
                                None => "".to_string()
                            };

                            extra_mounts.push(format!("type={},src={},dst={}{}", mount_type, src_expanded, dst_expanded, extra_options));
                        }
                    }
                }
            }

            // Process ports
            let mut ports: Vec<String> = Vec::new();
            if let Some(node) = command_entry.get("ports") {
                if let Ok(vec) = node.clone().into_array() {
                    for item in &vec {
                        ports.push(item.to_string());
                    }
                }
            }

            // Process flags
            let mut flags: Vec<String> = Vec::new();
            if let Some(node) = command_entry.get("flags") {
                if let Ok(vec) = node.clone().into_array() {
                    for item in &vec {
                        flags.push(item.to_string());
                    }
                }
            }

            let workdir_path = env::var("WORKDIR_PATH").unwrap_or_else(|_| "/workdir".to_owned());

            let config_struct = Configuration {
                image,
                name,
                dockerfile,
                root_path: path,
                workdir_path,
                flags,
                env_variables,
                build_args,
                extra_mounts,
                ports
            };

            return Ok(config_struct);
        } else {
            // Command not found in this config, try parent directory
            path.pop();
            return load_config(path, command);
        }
    } else {
        // No config file at this path, try parent directory
        if path.as_os_str().len() > 1 {
            path.pop();
            return load_config(path, command);
        } else {
            // Reached root without finding config
            return Err(Error::NoConfigFound { command: command.to_string() });
        }
    };
}

fn image_exists(image: &String) -> Result<bool, Error> {
    let status = Command::new("docker")
        .arg("image")
        .arg("inspect")
        .arg(image)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|e| Error::CommandError {
            cmd: format!("docker image inspect {}", image),
            reason: e.to_string()
        })?;

    Ok(status.success())
}

fn download_image(image: &String) -> Result<bool, Error> {
    println!("Downloading image: {}", image);
    let status = Command::new("docker")
        .arg("pull")
        .arg(image)
        .status()
        .map_err(|e| Error::CommandError {
            cmd: format!("docker pull {}", image),
            reason: e.to_string()
        })?;

    Ok(status.success())
}

fn container_exists(name: &String) -> Result<bool, Error> {
    let result = Command::new("docker")
        .arg("ps")
        .arg("-f")
        .arg(format!("name={}", name))
        .arg("--format")
        .arg("'{{.Names}}'")
        .output()
        .map_err(|e| Error::CommandError {
            cmd: format!("docker ps -f name={}", name),
            reason: e.to_string()
        })?;

    let output = String::from_utf8_lossy(&result.stdout)
        .to_string()
        .trim()
        .to_string()
        .replace("'", "");

    Ok(&output == name)
}

fn build_image(image: &String, dockerfile: &String, dockerfile_path: &PathBuf, workdir_path: &String, build_args: &Vec<String>) -> Result<bool, Error> {
    let dockerfile_path_str = dockerfile_path.to_str()
        .ok_or_else(|| Error::PathError("Dockerfile path contains invalid UTF-8".to_string()))?;

    println!("Building image: {}/{} -> {}", dockerfile_path_str, dockerfile, image);

    let mut docker_args: Vec<&str> = vec!["build"];

    let uid = get_current_uid();
    let result = get_user_by_uid(uid);
    let username: String = match result {
        None => "dev".to_string(),
        Some(user) => user.name().to_str().unwrap_or("dev").to_owned()
    };

    let uid_str = format!("uid={}", uid);
    let gid_str = format!("gid={}", get_current_gid());
    let username_str = format!("username={}", username.as_str());
    let workdir_path_str_arg = format!("workdir_path={}", workdir_path);

    docker_args.push("--build-arg");
    docker_args.push(&uid_str);
    docker_args.push("--build-arg");
    docker_args.push(&gid_str);
    docker_args.push("--build-arg");
    docker_args.push(&username_str);
    docker_args.push("--build-arg");
    docker_args.push(&workdir_path_str_arg);

    if !build_args.is_empty() {
        for item in build_args {
            docker_args.push("--build-arg");
            docker_args.push(item.trim());
        }
    }

    docker_args.push("-t");
    docker_args.push(image);
    docker_args.push("-f");
    docker_args.push(dockerfile);
    docker_args.push(dockerfile_path_str);

    println!("{} docker {}", "(executing)    ".bright_blue().bold(), docker_args.join(" "));

    let status = Command::new("docker")
        .current_dir(dockerfile_path_str)
        .args(docker_args)
        .status()
        .map_err(|e| Error::CommandError {
            cmd: format!("docker build -t {} -f {}", image, dockerfile),
            reason: e.to_string()
        })?;

    Ok(status.success())
}

fn run_command(command: &str, args: Vec<&str>, options: GlobalOptions) -> Result<bool, Error> {
    let current_path = std::env::current_dir()
        .map_err(|e| Error::PathError(format!("Failed to get current directory: {}", e)))?;
    let path_clone = current_path.clone();

    let c = load_config(path_clone, command)?;

    let root_path_str = c.root_path.to_str()
        .ok_or_else(|| Error::PathError("Root path contains invalid UTF-8".to_string()))?;

    println!("{} {}/.contain.yaml", "(configuration)".blue().bold(), root_path_str);

    let relative_path = current_path.as_path().strip_prefix(root_path_str)
        .map_err(|_| Error::PathError(format!(
            "Current directory '{}' is not under root path '{}'",
            current_path.display(), root_path_str
        )))?;
    let relative_path_str = relative_path.to_str()
        .ok_or_else(|| Error::PathError("Relative path contains invalid UTF-8".to_string()))?;
    let absolute_current_path = format!("{}/{}", c.workdir_path, relative_path_str);
    let absolute_current_path_str = absolute_current_path.as_str();

    // Skip image checks for dry run mode
    if !options.dry_run {
        // Check if image exists locally
        if !image_exists(&c.image)? {
            // Try downloading it
            if !download_image(&c.image)? {
                // Otherwise, build it
                if !build_image(&c.image, &c.dockerfile, &c.root_path, &c.workdir_path, &c.build_args)? {
                    return Err(Error::ImageBuildFailed {
                        image: c.image.clone(),
                        dockerfile: format!("{}/{}", root_path_str, c.dockerfile)
                    });
                }
            }
        }
    }

    println!("{} {}", "(using image)  ".blue().bold(), c.image);

    if let Some(n) = c.name.clone() {
        // Skip container existence check for dry run
        if !options.dry_run && container_exists(&n)? {
            println!("{} {}", "(executing inside existing container)  ".blue().bold(), &n);
            docker_exec(absolute_current_path_str, c, options, n.as_str(), command, args);
            return Ok(true);
        } else {
            docker_run(absolute_current_path_str, c, options, command, args);
        }
    } else {
        docker_run(absolute_current_path_str, c, options, command, args);
    }

    Ok(true)
}

fn docker_run(current_dir: &str, c: Configuration, options: GlobalOptions, command: &str, args: Vec<&str>) {
    let uid = get_current_uid();
    let gid = get_current_gid();
    let uid_gid = format!("{}:{}", uid, gid);

    let mount = format!("type=bind,src={},dst={}", c.root_path.to_str().unwrap(), c.workdir_path);

    let mut docker_args :Vec<&str> = vec![
        "run"
    ];

    let name;

    if let Some(n) = c.name {
        if ! options.skip_name {
            name = n;
            docker_args.push("--name");
            docker_args.push(name.as_str());
        }
    };

    if ! options.run_as_root && ! c.flags.contains(&"root".to_string()) {
        docker_args.push("-u");
        docker_args.push(uid_gid.as_str());
    }

    if ! options.keep_container && ! c.flags.contains(&"k".to_string()) {
        docker_args.push("--rm");
    };

    if options.interactive || c.flags.contains(&"i".to_string()) {
        docker_args.push("-it");
    };

    if c.flags.contains(&"privileged".to_string()) {
        docker_args.push("--privileged");
    };

    docker_args.push("-w");
    docker_args.push(current_dir);

    let all_env_variables = [&c.env_variables[..], &options.cli_env_variables[..]].concat();

    if all_env_variables.len() > 0 {
        for i in 0..all_env_variables.len() {
            let item = &all_env_variables[i];
            docker_args.push("-e");
            docker_args.push(item.trim());
        }
    }

    // Mount workspace
    docker_args.push("--mount");
    docker_args.push(&mount);

    if c.extra_mounts.len() > 0 {
        for i in 0..c.extra_mounts.len() {
            let item = &c.extra_mounts[i];
            docker_args.push("--mount");
            docker_args.push(item);
        }
    }

    if ! options.skip_ports {
        if c.ports.len() > 0 {
            for i in 0..c.ports.len() {
                let item = &c.ports[i];
                docker_args.push("-p");
                docker_args.push(item);
            }
        }
    }

    docker_args.push(&c.image);

    // Binary to execute inside container
    docker_args.push(command);

    // Arguments to pass to binary inside container
    docker_args.extend(args);

    return execute_command(options, "docker", docker_args);
}

fn docker_exec(current_dir: &str, c: Configuration, options: GlobalOptions, name: &str, command: &str, args: Vec<&str>) {
    let uid = get_current_uid();
    let gid = get_current_gid();
    let uid_gid = format!("{}:{}", uid, gid);

    let mut docker_args :Vec<&str> = vec![
        "exec"
    ];

    docker_args.push("-it");

    if ! options.run_as_root && ! c.flags.contains(&"root".to_string()) {
        docker_args.push("-u");
        docker_args.push(uid_gid.as_str());
    }

    docker_args.push("-w");
    docker_args.push(current_dir);

    let all_env_variables = [&c.env_variables[..], &options.cli_env_variables[..]].concat();

    if all_env_variables.len() > 0 {
        for i in 0..all_env_variables.len() {
            let item = &all_env_variables[i];
            docker_args.push("-e");
            docker_args.push(item.trim());
        }
    }

    docker_args.push(name);

    // Binary to execute inside container
    docker_args.push(command);

    // Arguments to pass to binary inside container
    docker_args.extend(args);

    return execute_command(options, "docker", docker_args);
}

fn execute_command(options: GlobalOptions, command: &str, args: Vec<&str>) {
    if ! options.dry_run {
        println!("{} {} {}", "(executing)    ".bright_blue().bold(), command, args.join(" "));
        match Command::new(command)
                       .args(args)
                       .spawn()
                       .expect("Could not run the command")
                       .wait() {
                            Ok(status) => {
                                // if options.keep_container {
                                //     println!("{} {}", format!("(kept container)  ").green().bold(), "CONTAINER_ID");
                                // }
                                // if options.persist_image {
                                //     println!("{} {}", format!("(persisted changes to)  ").green().bold(), "IMAGE_ID");
                                // }
                                
                                match status.code() {
                                    Some(code) => exit(code),
                                    None       => exit(0)
                                }
                            },
                            Err(err) => println!("ERROR {:?}", err)
                        }
    } else {
        println!("{} {} {}", "(dry run)      ".yellow().bold(), command, args.join(" "));
    }
}