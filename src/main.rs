extern crate config;
extern crate colored;

#[macro_use] extern crate clap;

#[macro_use]
extern crate quick_error;

use std::process::{Command, Stdio};
use std::path::PathBuf;
use std::collections::HashMap;

use clap::{Arg, App, AppSettings};
use colored::*;

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
    dry_run: bool
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
        run_as_root: false
    };

    let matches = App::new("contain")
        .setting(AppSettings::TrailingVarArg)
        .setting(AppSettings::AllowLeadingHyphen)
        .setting(AppSettings::ArgRequiredElseHelp)
        .setting(AppSettings::DisableVersion)
        .version(crate_version!())
        .author("Jonathan Pettersson")
        .about("Runs your development tool inside a container")
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

fn load_config(mut path: PathBuf, command: &str) -> Option<(String, String, String, Vec<String>, Vec<String>, Vec<String>, Vec<String>)> {
    let path_clone = path.clone();
    let path_str = path_clone.as_path()
        .to_str()
        .unwrap();

    let full_path = format!("{}/{}", path_str, CONTAIN_FILENAME);
    let mut pending_config = config::Config::default();

    let result = pending_config
        .merge(config::File::with_name(&full_path));

    if let Ok(ref config) = result {
        if let Some(command_entry) = get_config_table(config, command) {
            let image = command_entry.get("image").unwrap()
                .clone()
                .into_str().unwrap();
            let dockerfile = command_entry.clone().get("dockerfile").unwrap()
                .clone()
                .into_str().unwrap();
            
            let mut env_variables: Vec<String> = Vec::new();
            if let Some(node) = command_entry.get("env") {
                let node_clone = node.clone();
                if let Ok(vec) = node_clone.into_array() {
                    let vec_string : Vec<String> = vec.into_iter()
                                                            .map(|value| value.into_str().unwrap())
                                                            .collect();

                    env_variables = vec_string;
                    // println!("{:?}", vec_string);
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

                            let extra_options = match obj.get("options") {
                                Some(s) => format!(",{}", s.to_string()),
                                None => "".to_string()
                            };

                            extra_mounts.push(format!("type={},src={},dst={}{}", mount_type, src, dst, extra_options));
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

            let tpl = (image, dockerfile, path_str.to_string(), flags, env_variables, extra_mounts, ports);

            return Some(tpl);
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

fn get_user() -> (String, String) {
    let uid_output = Command::new("id")
                     .arg("-u")
                     .output()
                     .expect("failed to execute process 'id -u'");

    let gid_output = Command::new("id")
                  .arg("-g")
                  .output()
                  .expect("failed to execute process 'id -g'");

    let uid = String::from_utf8_lossy(&uid_output.stdout)
        .to_string()
        .trim()
        .to_string();
    let gid = String::from_utf8_lossy(&gid_output.stdout)
        .to_string()
        .trim()
        .to_string();

    (uid, gid)
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

fn build_image(image: &String, dockerfile: &String, dockerfile_path: &String) -> bool {
    println!("Building image: {}/{} -> {}", dockerfile_path, dockerfile, image);
    let status = Command::new("docker")
        .arg("build")
        .arg("-t")
        .arg(image)
        .arg("-f")
        .arg(dockerfile)
        .arg(dockerfile_path)
        .status()
        .expect("failed to execute process 'docker pull IMAGE'");

        status.success()
}

fn run_command(command: &str, args: Vec<&str>, options: GlobalOptions) -> Result<bool, Error> {
    let current_path = std::env::current_dir().unwrap();
    let path_clone = current_path.clone();
    let current_dir = current_path.as_path().to_str().unwrap();

    let (uid, gid) = get_user();
    let uid_gid = format!("{}:{}", uid, gid);
    
    let mut env_str: String = "".to_owned();
   
    println!("{} {}/.contain.yaml", format!("(configuration)").blue().bold(), path_clone.to_str().unwrap());
    if let Some((image, dockerfile, dockerfile_path, flags, env_variables, extra_mounts, ports)) = load_config(path_clone, command) {

        // Check if image exists locally
        if ! image_exists(&image) {
            // Try downloading it
            if ! download_image(&image) {
                // Otherwise, build it
                if ! build_image(&image, &dockerfile, &dockerfile_path) {
                    panic!("Unable to build docker image: {} with dockerfile: {}/{}", image, dockerfile_path, dockerfile);
                }
            }
        }

        println!("{} {}", format!("(using image)  ").blue().bold(), image);

        let mount = format!("type=bind,src={},dst=/workdir", current_dir);

        let mut docker_args :Vec<&str> = vec![
            "run"
        ];

        if ! options.run_as_root && ! flags.contains(&"root".to_string()) {
            docker_args.push("-u");
            docker_args.push(uid_gid.as_str());
        }

        if ! options.keep_container && ! flags.contains(&"k".to_string()) {
            docker_args.push("--rm");
        };

        if options.interactive || flags.contains(&"i".to_string()) {
            docker_args.push("-it");
        };


        if env_variables.len() > 0 {
            let env_string: String = env_variables
                                .into_iter()
                                .map(|s| format!("-e \"{}\"", s))
                                .collect();
            
            env_str.push_str(env_string.clone().as_str());
            docker_args.push(env_str.as_str());
        }
        

        // Mount workspace 
        docker_args.push("--mount");
        docker_args.push(&mount);

        if extra_mounts.len() > 0 {
            for i in 0..extra_mounts.len() {
                let item = &extra_mounts[i];
                docker_args.push("--mount");
                docker_args.push(item);
            }
        }

        if ports.len() > 0 {
            for i in 0..ports.len() {
                let item = &ports[i];
                docker_args.push("-p");
                docker_args.push(item);
            }
        }

        docker_args.push(&image);
        
        // Binary to execute inside container
        docker_args.push(command);

        // Arguments to pass to binary inside container
        docker_args.extend(args);


        if ! options.dry_run {
            println!("{} docker {}", "(executing)    ".bright_blue().bold(), docker_args.join(" "));
            match Command::new("docker").args(docker_args).spawn().expect("Could not run the command").wait() {
                Ok(_result) => {
                    if options.keep_container {
                        println!("{} {}", format!("(kept container)  ").green().bold(), "CONTAINER_ID");
                    }
                    if options.persist_image {
                        println!("{} {}", format!("(persisted changes to)  ").green().bold(), "IMAGE_ID");    
                    }
                },
                Err(err) => println!("{:?}", err)
            }
        } else {
            println!("{} docker {}", "(dry run)      ".yellow().bold(), docker_args.join(" "));
        }
    }else{
        return Err(Error::DockerError(format!("No docker image found for '{}' in .contain.yaml or any path above!", command).red()));
    }

    return Ok(true);
}
