## Contain your dev environments

Contain is an CLI tool that transparently runs your development tools inside docker containers. Configure contain to run your favorite development tool and you will get all the benefits of containerization, while mantaining the dev workflow you are used to.

### Project status

**Experiemental pre-release**

Disclaimer: The functionality and stability of this tool have been validated for a certain use-case. However, this is an early release of an experimental tool. Many aspects of the project will need to be improved before you can consider this a production-ready project: 

* Documentation
* Examples
* Automated tests
* Clean up and refactor contain core

If you are still interested (and brave), take a look at the examples directory for concrete examples of how you can use the tool. Also, if you are really curious, take a look at the INTERNALS.md file for some details of how it works.

### Rationale 

Containers have had a huge impact on how we build and deploy software. However, the development environment is typically still a hand-crafted workstation which contains certain versions of programming languages, tools and other programs. To make matters worse, we rarely document / automate how we configure our dev environments. It's especially painful to manage all these dependencies when collaborating with others on a project with multiple technologies.

In essence, the typical development environment is: 
* Hand-crafted & time-consuming to reproduce
* Unique
* Stateful
* Undocumented

Contain is a small CLI tool that aims to improve this situation by moving the dev environment into docker containers. With contain you can containerize a large part of you development environment. You can even share it with collaborators.

Benefits of contain:
* Reproducible dev environments
* No manual installation of development tools
* Standardized dev environments across you team

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
