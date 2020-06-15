FROM ubuntu:18.04

ARG uid
ARG gid
ARG username
ARG workdir_path

RUN groupadd -g $gid -r $username
RUN useradd --no-log-init -m -u $uid -r -g $username $username

RUN echo "$username ALL=(ALL) NOPASSWD: ALL" >> /etc/sudoers

WORKDIR $workdir_path