# MdOJ judger

A judger for cms.

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

## Improve reported accuracy

### Write your kernel infomation in the config

### Use tickless kernel

Due to the natural of Cgroup V2, we cannot use ``cpuacct.usage`` instead of ``cpu.stat``.

In ``cpu.stat`` provide the total cpu resource comsume since the creation, which was collected when cfs rescheduling process(it make a window span for accounting).

As the result, if we use tickless kernel, we can eliminate the window span.

### Config cpu to always work on base-clock

