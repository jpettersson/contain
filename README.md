## Contain your dev environments

Contain is an CLI tool that transparently runs your development tools inside docker containers. Configure contain to run your favorite development tool and you will get all the benefits of containerization, while mantaining the dev workflow you are used to.

### Project status

**Experiemental pre-release**

**Disclaimer:** The functionality and stability of this tool have been validated for a specific use-case: enable a shared standardized dev environment across machines in a small team. However, this is an early release of an experimental tool. Use at your own risk. Future version will most likely include breaking changes. Many aspects of the project will need to be improved before you can consider this a production-ready project: 

* Documentation
* More examples
* Automated tests
* Clean up and refactor code

If you are still interested (and brave), take a look at the examples directory for concrete examples of how you can use the tool. Also, if you are really curious, take a look at the INTERNALS.md file for some details of how it works.

### Rationale 

Containers have had a positive impact on how we build and deploy software. However, the typical development environment is still a hand-crafted workstation which contains certain versions of programming languages, tools and other programs. To make matters worse, we rarely document / automate how we configure our dev environments. It's especially painful to manage all these dependencies when collaborating with others on a project with multiple technologies.

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

### Installation

### Arch Linux packagee

[contain](https://aur.archlinux.org/packages/contain/)

#### Build with cargo

You can build the binary from scratch easily with cargo. Place the binary in a dir that's included in your `$PATH`.

```
cargo build --release --frozen --all-targets
```

### Collaboration

Collaboration is highly welcomed! I'm keeping a list of the most relevant bugs, features and todos up to date in the issues. Take a look and feel free to ping me in case you would like work on this.
