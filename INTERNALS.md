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