FROM archlinux

RUN pacman --noconfirm -Syu git base-devel rustup && \
    useradd -m aur && \
    echo 'aur ALL=(ALL) NOPASSWD: ALL' >> /etc/sudoers

USER aur
WORKDIR /home/aur

RUN rustup update stable

# Copies your code file from your action repository to the filesystem path `/` of the container
COPY entrypoint.sh /entrypoint.sh

# Code file to execute when the docker container starts up (`entrypoint.sh`)
ENTRYPOINT ["/entrypoint.sh"]