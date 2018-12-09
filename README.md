## Automatic containerized development environments

Use docker for project local development environments, but pass through commands as if your tools were installed globally.

### Examples
`contain yarn ...`
`contain mvn package`

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
* Support extra mountpoints? 
  - Solve the issue with global repos (~/.lein ~/.m2 etc) and let them live in the project root directory instead
* Support ENV variables

### Testing todo:
* Ensure required environment variables are readable
* Ensure -p and -k flags work as expected
* Ensure the run, pull build strategy works
* Test important permutations of the config file format
* Test config file validation