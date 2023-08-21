local socket = require("socket")

local ip,resolve = socket.dns.toip("google.com")

if ip ~= nil then
    print("connected")
elseif resolve ~= nil then
    print("connected")
end
