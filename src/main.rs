extern crate config;
extern crate colored;
extern crate users;
extern crate shellexpand;
extern crate semver;

#[macro_use] extern crate clap;

#[macro_use]
extern crate quick_error;

use std::process::{Command, Stdio, exit};
use std::path::PathBuf;
use std::collections::HashMap;
use std::env;

use clap::{Arg, App, AppSettings};
use colored::*;
use users::{get_user_by_uid, get_current_uid, get_current_gid};
use semver::Version;

quick_error! {
    #[derive(Debug)]
    pub enum Error {
        DockerError(descr: ColoredString) {
            description(descr)
            display("Error: {}", descr)
        }
        UnsupportedParameters(descr: ColoredString) {
            description(descr)
            display("Error: {}", descr)
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
        .version(crate_version!())
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
                    _ => return Err(Error::UnsupportedParameters(format!("Unsupported contain flag {}", command).red()))
                }
                num_program_flags += 1;
                flag = args[num_program_flags-1];
            }

            if num_program_flags > 0 {
                let mut mut_args = args.clone();
                return run_command(args[num_program_flags-1], mut_args.drain(num_program_flags..).collect(), options);
            }else{
                return run_command(command, args, options);
            }

        }else{
            return run_command(command, vec![], options);
        }
    }else{
        // This always happens because clap-rs triggers help if no command is passed..
        // TODO: Get rid of this else branch.

        return Ok(true);
    }
}

fn get_config_table(config: &config::Config, command: &str) -> Option<HashMap<String, config::Value>> {
    if let Ok(array) = config.get_array("images") {
        for node in &array {
            let table = &node.clone().into_table().unwrap();

            if let Ok(string) = table.get("commands").unwrap().clone().into_str() {
                if string == command || string == "any" {
                    return Some(table.clone())
                }
            }else if let Ok(entries) = table.get("commands").unwrap().clone().into_array() {
                for entry in &entries {
                    let entry_string = entry.clone().into_str().unwrap();
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

fn load_config(mut path: PathBuf, command: &str) -> Option<Configuration> {

    let path_clone = path.clone();
    let path_str = path_clone.as_path()
        .to_str()
        .unwrap();

    env::set_var("CONTAIN_ROOT_PATH", path_str);

    let full_path = format!("{}/{}", path_str, CONTAIN_FILENAME);
    let mut pending_config = config::Config::default();

    let result = pending_config
        .merge(config::File::with_name(&full_path));

    if let Ok(ref config) = result {

        let min_version:Option<String> = match config.get("contain_min_version") {
            Ok(n) => Some(n),
            Err(_) => None
        };

        if let Some(v) = min_version {
            if Version::parse(crate_version!()) < Version::parse(&v) {
                let required_version = &v.as_str();
                let current_version = &crate_version!();

                println!("ERROR: .contain.yaml requires contain version >= {} (current version: {})", required_version, current_version);
                exit(1);
            }
        };

        if let Some(command_entry) = get_config_table(config, command) {
            
            let image = command_entry.get("image").unwrap()
                .clone()
                .into_str().unwrap();

            let name = match command_entry.get("name") {
                None => None,
                Some(n) => Some(n.clone().into_str().unwrap())
            };

            let dockerfile = command_entry.clone().get("dockerfile").unwrap()
                .clone()
                .into_str().unwrap();


            if let Some(node) = command_entry.get("var") {
                let node_clone = node.clone();
                if let Ok(vec) = node_clone.into_array() {
                    for i in 0..vec.len() {
                        let item = &vec[i];
                        if let Ok(obj) = item.clone().into_table() {
                            let var_name = obj.get("name").unwrap();
                            let var_cmd = obj.get("command").unwrap();

                            let var_name_string = var_name.to_string();
                            let var_cmd_string = shellexpand::env(&var_cmd.to_string()).unwrap().into_owned();

                            let result = Command::new("sh")
                                        .arg("-c")
                                        .arg(var_cmd_string)
                                        .output()
                                        .expect("Failed to execute process: sh");

                            let output = String::from_utf8_lossy(&result.stdout)
                                .to_string()
                                .trim()
                                .to_string();

                            env::set_var(var_name_string, output);
                        }
                    }
                }
            }


            let mut env_variables: Vec<String> = Vec::new();
            if let Some(node) = command_entry.get("env") {
                let node_clone = node.clone();
                if let Ok(vec) = node_clone.into_array() {
                    let vec_string : Vec<String> = vec.into_iter()
                                                            .map(|value| value.into_str().unwrap())
                                                            .map(|value| shellexpand::env(&value).unwrap().into_owned())
                                                            .collect();
                    env_variables = vec_string;
                }
            }

            let mut build_args: Vec<String> = Vec::new();
            if let Some(node) = command_entry.get("build_args") {
                let node_clone = node.clone();
                if let Ok(vec) = node_clone.into_array() {
                    let vec_string : Vec<String> = vec.into_iter()
                                                            .map(|value| value.into_str().unwrap())
                                                            .map(|value| shellexpand::env(&value).unwrap().into_owned())
                                                            .collect();
                                                            build_args = vec_string;
                }
            }

            let mut extra_mounts: Vec<String> = Vec::new();
            if let Some(node) = command_entry.get("mounts") {
                let node_clone = node.clone();
                if let Ok(vec) = node_clone.into_array() {
                    for i in 0..vec.len() {
                        let item = &vec[i];
                        if let Ok(obj) = item.clone().into_table() {
                            let mount_type = obj.get("type").unwrap();
                            let src = obj.get("src").unwrap();
                            let dst = obj.get("dst").unwrap();

                            let src_string = src.to_string();
                            let dst_string = dst.to_string();

                            let src_expanded = shellexpand::env(&src_string).unwrap();
                            let dst_expanded = shellexpand::env(&dst_string).unwrap();

                            let extra_options = match obj.get("options") {
                                Some(s) => format!(",{}", s.to_string()),
                                None => "".to_string()
                            };

                            extra_mounts.push(format!("type={},src={},dst={}{}", mount_type, src_expanded, dst_expanded, extra_options));
                        }
                    }
                }
            }

            let mut ports: Vec<String> = Vec::new();
            if let Some(node) = command_entry.get("ports") {
                let node_clone = node.clone();
                if let Ok(vec) = node_clone.into_array() {
                    for i in 0..vec.len() {
                        let item = &vec[i];
                        ports.push(item.to_string());
                    }
                }
            }

            let mut flags: Vec<String> = Vec::new();
            if let Some(node) = command_entry.get("flags") {
                let node_clone = node.clone();
                if let Ok(vec) = node_clone.into_array() {
                    for i in 0..vec.len() {
                        let item = &vec[i];
                        flags.push(item.to_string());
                    }
                }
            }

            let workdir_path = match env::var("WORKDIR_PATH") {
                Ok(p) => p,
                Err(_) => "/workdir".to_owned()
            };

            let config_struct = Configuration {
                image: image,
                name: name,
                dockerfile: dockerfile,
                root_path: path,
                workdir_path: workdir_path,
                flags: flags,
                env_variables: env_variables,
                build_args: build_args,
                extra_mounts: extra_mounts,
                ports: ports
            };

            return Some(config_struct);
        }else{
            path.pop();
            return load_config(path, command);
        }
    } else {
        if path.as_os_str().len() > 1 {
            path.pop();
            return load_config(path, command);
        }else{
            // No config file found in tree
            return None
        }
    };
}

fn image_exists(image: &String) -> bool {
    let status = Command::new("docker")
        .arg("image")
        .arg("inspect")
        .arg(image)
        .stdout(Stdio::null())
        .status()
        .expect("failed to execute process 'docker inspect IMAGE'");

        status.success()
}

fn download_image(image: &String) -> bool {
    println!("Downloading image: {}", image);
    let status = Command::new("docker")
        .arg("pull")
        .arg(image)
        .status()
        .expect("failed to execute process 'docker pull IMAGE'");

        status.success()
}

fn container_exists(name: &String) -> bool {

    let result = Command::new("docker")
        .arg("ps")
        .arg("-f")
        .arg(format!("name={}", name))
        .arg("--format")
        .arg("'{{.Names}}'")
        .output()
        .expect("Failed to execute process: docker");

    let output = String::from_utf8_lossy(&result.stdout)
        .to_string()
        .trim()
        .to_string()
        .replace("'", "");

    return &output == name;
}

fn build_image(image: &String, dockerfile: &String, dockerfile_path: &PathBuf, workdir_path: &String, build_args: &Vec<String>) -> bool {
    let dockerfile_path_str = dockerfile_path.to_str().unwrap();

    println!("Building image: {}/{} -> {}", dockerfile_path_str, dockerfile, image);

    let mut docker_args :Vec<&str> = vec![
            "build"
    ];

    let uid = get_current_uid();
    let result = get_user_by_uid(uid);
    let username:String = match result {
        None => "dev".to_string(),
        Some(user) => user.name().to_str().unwrap().to_owned()
    };

    let uid_str = format!("uid={}", uid);
    let gid_str = format!("gid={}", get_current_gid());
    let username_str = format!("username={}", username.as_str());
    let workdir_path_str = format!("workdir_path={}", workdir_path);

    docker_args.push("--build-arg");
    docker_args.push(&uid_str);
    docker_args.push("--build-arg");
    docker_args.push(&gid_str);
    docker_args.push("--build-arg");
    docker_args.push(&username_str);
    docker_args.push("--build-arg");
    docker_args.push(&workdir_path_str);

    if build_args.len() > 0 {
        for i in 0..build_args.len() {
            let item = &build_args[i];
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
        .expect("failed to execute process 'docker pull IMAGE'");

        status.success()
}

fn run_command(command: &str, args: Vec<&str>, options: GlobalOptions) -> Result<bool, Error> {
    let current_path = std::env::current_dir().unwrap();
    let path_clone = current_path.clone();

    if  let Some(c) = load_config(path_clone, command) {
        println!("{} {}/.contain.yaml", format!("(configuration)").blue().bold(), c.root_path.to_str().unwrap());

        let current_path = current_path.as_path().strip_prefix(c.root_path.to_str().unwrap()).unwrap();
        let current_path_str = current_path.to_str().unwrap();
        let absolute_current_path = format!("{}/{}", c.workdir_path, current_path_str);
        let absolute_current_path_str = absolute_current_path.as_str();

        // Check if image exists locally
        if ! image_exists(&c.image) {
            // Try downloading it
            if ! download_image(&c.image) {
                // Otherwise, build it
                if ! build_image(&c.image, &c.dockerfile, &c.root_path, &c.workdir_path, &c.build_args) {
                    panic!("Unable to build docker image: {} with dockerfile: {}/{}", c.image, c.root_path.to_str().unwrap(), c.dockerfile);
                }
            }
        }

        println!("{} {}", format!("(using image)  ").blue().bold(), c.image);

        if let Some(n) = c.name.clone() {
            if container_exists(&n) {
                println!("{} {}", format!("(executing inside existing container)  ").blue().bold(), &n);
                docker_exec(absolute_current_path_str, c, options, n.as_str(), command, args);
                return Ok(true);
            }else{
              docker_run(absolute_current_path_str, c, options, command, args);
            }
        }else{
            docker_run(absolute_current_path_str, c, options, command, args);
        }

    }else{
        return Err(Error::DockerError(format!("No docker image found for '{}' in .contain.yaml or any path above!", command).red()));
    }

    return Ok(true);
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