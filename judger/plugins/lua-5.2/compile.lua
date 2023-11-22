local src_txt=io.read("*all")

local src=io.open("/src/source.txt","w")
if src==nil then
    print("4: unknown error")
else
    print("1: writing to /src/source.txt")
    src:write(src_txt)
end
