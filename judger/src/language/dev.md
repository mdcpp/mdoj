## Module Layout

## Design

### Plugin

Plugin extend judger to support more (programing)language, a result artifact is a tarball named `*.lang`.

#### Structure in tarball


The c-11 example tarball look like this:
![image](https://github.com/mdcpp/mdoj/assets/30045503/2a50af00-1350-4f3b-a231-9a1ab1e12bf6)

Please be noted that spec.toml is required.

#### Spec.toml

```toml
file = "/code.c"
fs_limit = 3145728 # number of byte the whole process(compile+judge) is allowed to write
info = "gcc 13.2.0 (G++)"
extension = "c"
name = "c-11"
id = "7daff707-26b5-4153-90ae-9858b9fd9619" # you can generate it randomly(https://www.uuidgenerator.net)

[compile]
command = ["/usr/bin/cc","-x", "c", "code.c", "-lm", "-o", "/execute"]
kernel_mem = 1 # number of kernel space memory limit in byte
memory = 1 # number of total memory limit in byte
user_mem = 1 # number of userspace memory limit in byte
rt_time = 1 # number of non-preemptible execution time in nanosecond
cpu_time = 1 # number of preemptible execution time in nanosecond
total_time = 1 # # number of non-preemptible execution time in nanosecond
walltime = 1 # number of time in **milliseconds**(realtime, it count even scheduler didn't dispatch any time for the task)/

[judge]
command = ["/execute"]
kernel_mem = 1 # number of kernel space memory limit in byte
rt_time = 1 # number of non-preemptible execution time in nanosecond
cpu_time = 1 # number of preemptible execution time in nanosecond
total_time = 1 # # number of non-preemptible execution time in nanosecond
memory_multiplier = 1.0 # multiplier for total memory limit in byte
cpu_multiplier = 1.0  # multiplier for total cpu execution limit in nanosecond
output = 1 # max output in byte
walltime = 1 # number of time in **milliseconds**(realtime, it count even scheduler didn't dispatch any time for the
```
