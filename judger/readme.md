# MdOJ judger

A judger for cms.

## How to integrate it to web backend?

See ``proto/plugin.proto``, use grpc to communicate with it.

## How to build it?

Install just, clone this git repo, and run ``just install-deps-debian``, ``just build-nsjail``, ``just build-plugin``.

After that, build this crate(``cargo build --release``).

## System Requirement

1. CGroupv2 support
2. Linux kernel 5.14 or later

## How to develop new language supports?

``language support`` are called ``plugin`` in this project.

Follow guide in ``/plugins/readme.md``

## Setup in production

We recommand docker for production. However, other platforms compatible with OCI-runtime is also supported.

Note that the container require ownership of namespace(either user or root), cgroup to be functional.

Also, consider setting following critical config before production:

```
platform.cpu_time_multiplier: 2 to give each request twice the original time limit to execute, useful if you have multiple judger with different hardware setup.
platform.available_memory: amount of memory that user submitted request and output buffer can use, $($all_memory-1024*1024*1024) is recommanded, out of memory would result in runtime error in the request result.
```

### Docker

Run with default config in privileged container, judger would generate config automatically.

### Podman & Kubernetes

both user namespace and user cgroup is available to K8s, consider using it.

Following config should be modified:

```
runtime.root_cgroup: cgroup path, default: ``/sys/fs/cgroup/mdoj``
nsjail.rootless: use user namespace if user namespace is available;
```

~~I don't use K8s, don't ask me how to write config.~~

## Improve reported accuracy

### Use cgroup v1

Cgroup v1 provide a subsystem called cpuacct(cpu accounting), it is more accurate than cpu subsystem(We use it in cgroup v2).

Change ``nsjail.cgroup_version`` to "v1" in the config file to switch to cgroup v1.

### Write your kernel infomation in the config

We return accuracy along with each response, although provide kernel infomation can not improve accuracy, it can calibrate accuracy returned.

### Use tickless kernel

Due to the natural of Cgroup V2, we cannot use ``cpuacct.usage`` instead of ``cpu.stat``, which result in window span when cfs reschedule, if we use tickless kernel, we can eliminate the window span.

### Config cpu to always work on baseclock

We measures both user space and kernel space execution time of the process, a stable process clock can make result more stable. 


