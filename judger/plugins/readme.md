# Develop a module for new language support(plugin)

`rlua-54` and `c-11` is the example plugin

|name|application| 
|:-:|:-|
|rlua-54|interpreter language|
|c-11|compile language|

an language support it's just a tar file containing a whole filesystem, including `spec.toml` at root of tar file.

## How to generate such tar file?

use `docker export`

```
docker export ___ > c-11.lang
```

## spec.toml

Not all field is required, the minimal field required is show as example in `rula-54`.

the full spec.toml is show below:

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
walltime = 1 # number of time in **milliseconds**(realtime, it count even scheduler didn't dispatch any time for the task)/

```
