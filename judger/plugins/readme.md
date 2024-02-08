# Develop a module for new language support

`rlua-54` and `rlua-54` is the example plugin

## How it works?

Compile Stage:

1. Start compile command in jail
2. Daemon write source code to jail's stdout, and close pipe
3. ``Compile`` subprocess exit

Execute Stage:

4. Start execute command in jail
5. Daemon write input to jail's stdin
6. ``Execute`` subprocess exit
7. Daemon exam the exit status of subprocess
8. Daemon exam the stdout of ``Execute`` subprocess.

If you would like to log, write following string(utf-8 only) to stderr in compile stage:

```log
<Level><Message>
```

Don't append ``\n`` in the end of the stdout, and it should never be ``\r\n``.

LEVEL should be number 1-6, would be interpreted as TRACE, DEBUG, INFO, WARN, ERROR.

The return code of non-zero would be considered Compile Error.

## How to develop a container for language support

One of the recommanded way is with ``docker export``.

Program your own docker image and run:
 
```shell
# if docker is not in rootless mode you need add `sudo` in front of docker command
docker export ${id of your docker container} | tar -C plugins/${plugin name}/rootfs -xvf -
```

finish spec.toml like this
```ini
# memory in byte, time in microsecond
info = "A Lua 5.4 runtime build both real workloads and sandbox tests"
extension = "lua" # same extension means same language
name = "rlua-54" # must be same as dictionary name
uid = "1c41598f-e253-4f81-9ef5-d50bf1e4e74f" # be sure it's unique

[compile]
lockdown = true
command = ["/rlua-54","compile"]
kernel_mem = 67108864
user_mem = 268435456
rt_time = 1000000
cpu_time = 10000000
total_time = 10000000

[judge]
command = ["/rlua-54","execute"]
kernel_mem = 67108864
multiplier_memory = 6 # user_mem
rt_time = 1000000
multiplier_cpu = 3 # cpu_time
```

## Troubleshooting

### Execute jail recieve SIGKILL with no reason

Possiblility:

1. Execute jail was lockdown(cannot write anything) for security reason, and the process in which try to open a file with write flag('w' in name of chmod).

