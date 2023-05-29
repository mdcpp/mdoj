local src=io.open("/src/source.txt")
if src==nil then
    print("ISE: unknown")
else
    local context=src:read("*a")
    load(context)()
end
