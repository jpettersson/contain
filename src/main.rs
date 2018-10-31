extern crate config;

#[macro_use] extern crate clap;
use clap::{Arg, App, AppSettings};

use std::process::{Command, Stdio};
use std::path::PathBuf;
use std::collections::HashMap;

const COMMAND: &str = "command";
const ARGS: &str = "args";
const CONTAIN_FILENAME: &str = ".contain.yaml";

fn main() {
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
            run_command(command, args);
        }else{
            run_command(command, vec![]);
        }
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

    // No matching command was found in this document
    None
}

fn load_config(mut path: PathBuf, command: &str) -> Option<(String, String, String)> {
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
            let tpl = (image, dockerfile, path_str.to_string());

            return Some(tpl);
        }else{
            println!("else");
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

fn execute(cmd: &mut Command) {
    if let Err(err) = cmd.spawn().expect("Could not run the command").wait() {
        println!("{:?}", err);
    }
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

fn run_command(command: &str, args: Vec<&str>) {
    let current_path = std::env::current_dir().unwrap();
    let path_clone = current_path.clone();
    let current_dir = current_path.as_path().to_str().unwrap();

    let (uid, gid) = get_user();
    let uid_gid = format!("{}:{}", uid, gid);

    if let Some((image, dockerfile, dockerfile_path)) = load_config(path_clone, command) {

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

        let mount = format!("type=bind,src={},dst=/workdir", current_dir);
        let mut docker_args = vec![
            "run",
            "-u",
            uid_gid.as_str(),
            "--rm",
            "--mount",
            &mount,
            &image,
            command];

        docker_args.extend(args);
        println!("docker {:?}", docker_args);

        execute(Command::new("docker").args(docker_args));
    }else{
        panic!("No docker image found for '{}' in .contain.yaml or any path above!", command);
    }

}
