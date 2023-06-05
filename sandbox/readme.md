# MdOJ sandbox

A sandbox for cms.

## How to integrate it to web backend?

See ``proto/plugin.proto``, use grpc to communicate with it.

## How to build it?

Install just, clone this git repo, and run ``just install-deps-debian``, ``just build-nsjail``.

After that, build this crate(``cargo build --release``).

## System Requirement

1. CGroupv2 support
2. Linux kernel 5.14 or later

## How to develop a plugin?

Follow guide in ``/plugins/readme.md``

## Setup

### Standalone

### Docker

config.toml need to specify the host path instead of the container path.
