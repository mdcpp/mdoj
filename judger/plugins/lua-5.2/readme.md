# Example plugin

```toml
# memory in byte, time in microsecond
info = "LuaJIT 5.2"
extension = "lua" # same extension means same language
name = "lua-5.2" # must be same as dictionary name
uid = "f060f3c5-b2b2-46be-97ba-a128e5922aee" # be sure it's unique

[compile]
lockdown = true
command = ["lua", "/compile.lua"]
kernel_mem = 67108864
user_mem = 268435456
rt_time = 1000000
cpu_time = 10000000
total_time = 10000000

[judge]
command = ["lua", "/execute.lua"]
kernel_mem = 67108864
multiplier_memory = 6 # user_mem
rt_time = 100000
multiplier_cpu = 3 # cpu_time
```