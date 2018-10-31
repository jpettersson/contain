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

### Rough edges
* Ensure users are the same in host system and containers to avoid file permission issues