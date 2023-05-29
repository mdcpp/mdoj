local src_txt=io.read("*a")

local src=io.open("/src/source.txt","w")
if src==nil then
    print("ISE: unknown")
else
    src:write(src_txt)
end
