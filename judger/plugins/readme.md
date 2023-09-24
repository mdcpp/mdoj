# Develop a module for new language support

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
docker export ${id of your docker container} | tar -C plugins/${plugin name}/rootfs -xvf -
```

finish spec.toml like this
```ini
description = "LuaJIT 5.2"
extension = "lua" # same extension means same language
name = "lua-5.2" # must be same as dictionary name
uuid = "f060f3c5-b2b2-46be-97ba-a128e5922aee" # be sure it's unique

[compile]
command = ["lua", "/compile.lua"] # command to run when compile
# in byte
kernel_mem = 16777216
user_mem = 33554432
# in microsecond
rt_time = 1000000
cpu_time = 10000000

[execute]
command = ["lua", "/execute.lua"] # command to run when execute
kernel_mem = 16777216
memory_multiplier = 6 # user_mem
rt_time = 100000
cpu_multiplier = 3 # cpu_time
```

## Troubleshooting

### Execute jail recieve SIGKILL with no reason

Possiblility:

1. Execute jail was lockdown(cannot write anything) for security reason, and the process in which try to open a file with write flag('w' in name of chmod).

