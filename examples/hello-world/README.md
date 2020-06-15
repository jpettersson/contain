# Hello World

This example shows how to configure contain to run commands inside a dev container.

## 1. Install contain
Install the contain binary to your path according to instructions in README.md.

## 2. Run a command

To run a simple command inside the container, try: 

```
contain echo hello world
```

Contain will: 

1. Look for the docker image specified in .contain.yaml
2. Download the image if it exists in a registry
3. Otherwise, build it from the Dockerfile
4. Start the container and execute `echo hello world` inside the container
5. Exit the container and propagate the internal exitcode to the outside

## 3. Run an interactive shell

Contain has the ability to run interactive sessions. To get an interactive shell inside the container, run:

```
contain -i /bin/bash
```

## 4. Run multiple commands inside the same container

Contain can run multiple commands inside the same container. The first command will be executed using `docker run` and subsequent command to the same container will use `docker exec`.

Keep the interactive container from the previous step running. From a new terminal, try: 

```
contain ps aux
```

This will display both processes running inside the container.

## Dockerfile

Contain reads some useful information from the environment and injects it as docker build args that are available during build. This allows us to specify a small amount of boilerplate in our Dockerfile to customize the running container to better match the host system:

* Ensure the path inside the container is identical to the path outside where contain was executed from
* Ensure that the docker process is running using the same UID and GID as the user executing contain. This helps ensure that file permissions are kept sane.

Take a look at the Dockerfile to see which build args are available and an example of the boilerplate that can be used to configure a dev image to take full advantage of them.
