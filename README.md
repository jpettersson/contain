## Contain your dev environments

Contain is a CLI tool that seamlessly runs your development tools inside docker containers. Configure contain to run your favorite development tool and you will get all the benefits of containerization, while mantaining the workflow you are used to.

EXAMPLE GIF

### How it works

1. The contain cli acts as a proxy, it takes a command and executes it inside a docker container:

```bash
contain ls
```

2. When started, contain will look for a config file called `.contain.yaml`. This file should live in the root of your project directory. The config files specifies which Docker image should be used to instantiate the container, as well as other standard docker parameters such as env variables, ports and volumes.

**Simple example: .contain.yaml**

```yaml
images:
  - image: "my-image:latest"
    dockerfile: Dockerfile
    commands: any
```

*Note: In this example the configuration specifies that all commands should execute inside containers created from the `my-image:latest` image.

3. In addition to the parameters defined in the `.contain.yaml` file, contain will mount the current directory to `/workdir` inside the container.

4. The Dockerfile referenced above could look like this

```Dockerfile
FROM ubuntu:18.04

# contain start
ARG uid
ARG gid
ARG username

RUN groupadd -g $gid -r $username
RUN useradd --no-log-init -m -u $uid -r -g $username $username

RUN echo "$username ALL=(ALL) NOPASSWD: ALL" >> /etc/sudoers

WORKDIR /workdir
# contain end
```

The contain specific section is needed to make sure that the process is executed with the same user permissions as the host system user. This ensures that there are no file permissions issues between the container and the host system. Additionally, since contain injects `uid`, `gid` and `username` from the host system, these variables are available to use in other sections of the Dockerfile.

### Rationale 

Containers have had a huge impact on how we build and deploy software. However, the development environment is typically still a hand-crafted workstation which contains certain versions of programming languages, tools and other programs. To make matters worse, we rarely document / automate how we configure our dev environments. It's especially painful to manage all these dependencies when collaborating with others on a project with multiple technologies.

In essence, the typical development environment is: 
* Hand-crafted & time-consuming to reproduce
* Unique
* Stateful
* Undocumented

Contain is a small CLI tool that aims to improve this situation using docker containers. With contain you can containerize a large part of you development environment. You can even share it with collaborators.

Benefits of contain:
* Reproducible dev environments
* No manual installation of development tools
* Standardized dev environments across you team
* Structured documentation for free

### Procedure
1. Look for `.contain.yaml` or walk the directory tree upwards until `.contain.yaml` file is found.
2. Look for the `contain ...` command key
2. If found, use the image specified
  * If not exists, build it from the Dockerfile specified
3. Start docker container:
  * Mount local directory
  * Delete after execution

### Features & todo:
* Write README & project examples
* Decide MVP scope
* -p flag: Implement logic for persisting images from running containers (to allow dependencies to be installed in images, etc)
* Display IP address of docker container for long running commands (web servers, etc)
* Detect & use docker-machine when needed
* Ensure containers shut down on termination signals
* Ensure that interactive scripts work (issue with lein repl quitting immidiately)
* Helper methods for building docker images?
* Get rid of all `.unwrap()` calls
* Support IDE plugins using docker exec (background processes)
  * Start a container in the background and execute rustc, leiningen, etc inside it.

Configuration (.contain.yaml)
* Support ENV variables in entire .contain.yaml file

Scaffolding Dockerfiles
* Specify template Dockerfiles in ~/.contain.yaml
* Include defaults in the documentation

### Testing todo:
* Ensure required environment variables are readable
* Ensure -p and -k flags work as expected
* Ensure the run, pull build strategy works
* Test important permutations of the config file format
* Test config file validation
