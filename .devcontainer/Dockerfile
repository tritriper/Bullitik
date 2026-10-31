# syntax=docker/dockerfile:1
#
# Image du devcontainer Bullitik, aussi utilisée par la CI Linux.
# Tout est épinglé pour qu'une reconstruction donne la même image :
#   - l'image de base par son digest ;
#   - les paquets apt par une date de l'archive snapshot.ubuntu.com ;
#   - rustup par sa version et son SHA-256, la toolchain par rust-toolchain.toml.

FROM ubuntu:24.04@sha256:fe81560990fae86dc4743202b2cb4d3df98d0bcf949173cecef15ce1127ef852

# Date de l'archive Ubuntu dont viennent les paquets (AAAAMMJJTHHMMSSZ).
# La changer, c'est mettre à jour tous les paquets d'un coup.
ARG UBUNTU_SNAPSHOT=20261030T000000Z

# - build : compilateur C/C++, cmake et clang pour whisper.cpp et bindgen ;
# - Vulkan : en-têtes, compilateur de shaders, pilotes Mesa (dont lavapipe,
#   Vulkan sur CPU, qui suffit pour compiler et tester sans GPU) ;
# - son : ALSA pour cpal, redirigé vers PulseAudio (WSLg) par /etc/asound.conf.
#
# L'archive snapshot est en HTTPS : on installe d'abord les certificats depuis
# l'archive courante, puis toutes les sources pointent vers le snapshot daté.
RUN apt-get update \
 && DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends ca-certificates \
 && sed -i "s#^URIs: http://\(archive\|security\)\.ubuntu\.com/ubuntu/\?\$#URIs: https://snapshot.ubuntu.com/ubuntu/${UBUNTU_SNAPSHOT}/#" \
      /etc/apt/sources.list.d/ubuntu.sources \
 && apt-get update \
 && DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends \
      build-essential cmake clang libclang-dev pkg-config \
      ca-certificates curl git sudo \
      libvulkan-dev glslc mesa-vulkan-drivers vulkan-tools \
      libasound2-dev libasound2-plugins alsa-utils pulseaudio-utils \
 && rm -rf /var/lib/apt/lists/*

COPY .devcontainer/asound.conf /etc/asound.conf

# L'utilisateur `ubuntu` (uid 1000) existe déjà dans l'image de base.
RUN echo "ubuntu ALL=(ALL) NOPASSWD:ALL" > /etc/sudoers.d/ubuntu && chmod 0440 /etc/sudoers.d/ubuntu
USER ubuntu
ENV RUSTUP_HOME=/home/ubuntu/.rustup \
    CARGO_HOME=/home/ubuntu/.cargo \
    PATH=/home/ubuntu/.cargo/bin:${PATH}

ARG RUSTUP_VERSION=1.29.2
ARG RUSTUP_SHA256=5be191e8c70a5007074254cb74d78dbfdd25cd5a61c27de7b8e5410122ec24f4
RUN curl --proto '=https' --tlsv1.2 -sSfL -o /tmp/rustup-init \
      "https://static.rust-lang.org/rustup/archive/${RUSTUP_VERSION}/x86_64-unknown-linux-gnu/rustup-init" \
 && echo "${RUSTUP_SHA256}  /tmp/rustup-init" | sha256sum -c - \
 && chmod +x /tmp/rustup-init \
 && /tmp/rustup-init -y --no-modify-path --profile minimal --default-toolchain none \
 && rm /tmp/rustup-init

# La toolchain du dépôt, installée dans l'image plutôt qu'au premier `cargo`.
COPY --chown=ubuntu rust-toolchain.toml /tmp/toolchain/
RUN cd /tmp/toolchain && rustup toolchain install && rm -rf /tmp/toolchain

# Les modèles vivent dans un volume monté ici (voir devcontainer.json).
WORKDIR /workspaces/Bullitik
