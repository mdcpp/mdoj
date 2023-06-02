local src=io.open("/src/source.txt")
if src==nil then   
else
    local context=src:read("*a")
    load(context)()
end
