# This example base-image is built from:
# https://github.com/archlinux/archlinux-docker
# The only modification is that it also includes the following packages:
# sed, gzip, pacman, systemd, procps, wget

FROM jpettersson/archlinux-base:20200910

# Contains provides the following information from the host system
# and provides them as build args
ARG uid=10000
ARG gid=10000
ARG username=dev

USER root

# Install some basic things that we need
RUN pacman --noconfirm -Suy sudo iputils procps-ng base-devel iproute2 meson gtest gmock nano openssh 

# Create a user with identical username, UID and GID as the host
# user. This way we won't have any permissions issues
# with files that are mounted from the host system and modified 
# inside the container
RUN groupadd -g $gid -r $username
RUN useradd --no-log-init -m -u $uid -r -g $username $username

RUN echo "$username ALL=(ALL) NOPASSWD: ALL" >> /etc/sudoers

### Install pacman deps (add your deps here)
RUN pacman --noconfirm -Syu jdk8-openjdk jdk11-openjdk jdk-openjdk clojure git maven

### Install pacaur
RUN mkdir /install-pacaur
RUN chown -R $username /install-pacaur && chgrp -R $username /install-pacaur
WORKDIR /install-pacaur

# Switch to run the remaining steps as the normal user
USER $username

RUN wget https://aur.archlinux.org/cgit/aur.git/snapshot/pod2man.tar.gz
RUN tar -xf pod2man.tar.gz
RUN cd pod2man && makepkg --noconfirm -si

RUN wget https://aur.archlinux.org/cgit/aur.git/snapshot/auracle-git.tar.gz
RUN tar -xf auracle-git.tar.gz
RUN cd auracle-git && makepkg --noconfirm -si

RUN wget https://aur.archlinux.org/cgit/aur.git/snapshot/pacaur.tar.gz
RUN tar -xf pacaur.tar.gz
RUN cd pacaur && makepkg --noconfirm -si

### Install AUR deps (add your deps here)
ENV EDITOR nano
ENV VISUAL nano
RUN pacaur --noconfirm --noedit -Syu rlwrap babashka-bin