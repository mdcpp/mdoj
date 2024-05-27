## Module Layout

## Prerequisite knowledge

### Control Group(linux)

> In Linux, control groups (cgroups for short) act like a resource manager for your system. It lets you organize processes into groups and set limits on how much CPU, memory, network bandwidth, or other resources they can use.

> cgroup is abbr for control group

In practice, linux kernel expose cgroup's interface by vfs.

To get started, you can follow [it article](https://access.redhat.com/documentation/zh-tw/red_hat_enterprise_linux/6/html/resource_management_guide/sec-creating_cgroups) from red hat to create one.

In this project, we use `cgroups_rs`, which is an abstraction over underlying vfs.
