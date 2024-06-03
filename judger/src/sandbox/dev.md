## Module Layout
- `error.rs`: collection of internal error
- `macro_.rs`: useful macro for async
- `process` module: collection of process, much like chain of responsibility
    - `nsjail.rs`: factory for building paramter for nsjail
    - `process.rs`: a process that hasn't run yet.
    - `corpse.rs`: a process that is dead and collect by parent process.
- `monitor` module: composite of different kind of monitors
    - `stat.rs`: types that foreign function pass as paramter to monitor
    - `hier.rs`:  provide infomation that which cgroup controller should be use
    - `wrapper.rs`: newtype wrapper for `cgroups_rs::Cgroup`, it contain low level logic that may differ between cgroup version one and two
    - `mem_cpu.rs`: monitor for resource usage which rely on cgroup to be functional
    - `output.rs`: buffer output of process(`man pipe`), check if output limit excessive
    - `walltime.rs`: check if programm take too long to complete(if a process refuse to consume cpu time)

## Prerequisite knowledge

### Namespace(linux)

> Namespace creates isolated environments for processes, where each process sees its own version of specific resources.

common namespace include:

|Namespace|Flag|Isolates|
|:-:|:-:|:-|
|Cgroup|CLONE_NEWCGROUP|Cgroup root directory|
|IPC|CLONE_NEWIPC|Inter process communication|
|Network|CLONE_NEWNET|Network stack, port|
|Mount|CLONE_NEWNS|Mount points|
|PID|CLONE_NEWPID|Process ID|
|User|CLONE_NEWUSER|uid, gid|

> Note that when running in container, unshare(CLONE_NEWUSER) is required and unshare(CLONE_NEWCGROUP) is not allowed.

We only call cgroup namespace directly in this project, implementing secured isolation is considered error-proned and hard to verify.

### Control Group(linux)

> In Linux, control groups (cgroups for short) act like a resource manager for your system. It lets you organize processes into groups and set limits on how much CPU, memory, network bandwidth, or other resources they can use.

In practice, linux kernel expose cgroup's interface by vfs.

To get started, you can follow [it article](https://access.redhat.com/documentation/zh-tw/red_hat_enterprise_linux/6/html/resource_management_guide/sec-creating_cgroups) from red hat to create one.

In this project, we use `cgroups_rs`, which is an abstraction over underlying vfs.

#### `subsystem`

subsystem is a building block of cgroup, each subsystem control one type of resource

common subsystem include:`cpuset`,`cpu`,`io`,`memory`,`hugetlb`,`pids`,`rdma`,`misc`.

#### Using Cgroup in from high level

First, let's creat a control group.
```
sudo cgcreate -a eason:eason -g cpuset,cpu,io,memory,pids:mdoj-test
```
This command create a control group own by user `eason` with subsystem

Once a cgroup owned by a user is created, you can write rust program and run under normal user.
```rust
let cgroup = Arc::new(
    CgroupBuilder::new("mdoj-test/1")
        .memory()
        .kernel_memory_limit(mem.kernel as i64)
        .memory_hard_limit(mem.user as i64)
        .memory_swap_limit(0)
        .done()
        .cpu()
        .quota(MONITOR_ACCURACY.as_nanos() as i64)
        .realtime_period(MONITOR_ACCURACY.as_nanos() as u64)
        .done()
        .set_specified_controllers(vec!["cpu","memory","pids"].into_iter().map(|x|x.to_string()).collect())
        .build(MONITER_KIND.heir())?,
);
```

> Please be aware of the use after check bug, possibly use `pidfd`.
Add a process by pid
```rust
cgroup.add_task(12345);
```

get cpu controller
```rust
let cpu_acct: &CpuAcctController = cgroup.controller_of().expect("No cpu controller attached!");
```

get stat
```rust
let usage = cpu_acct.cpuacct();
```

### nsjail

[Nsjail](https://github.com/google/nsjail) is a free and opensource command line tool provided by Google(It's not a google product, don't blame they if nsjail contain security flaw).

## Special project decision
