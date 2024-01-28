```toml
bind_address = "0.0.0.0:8081"
log_level = 0
opentelemetry = false

[database]
path = "database/backend.sqlite"
salt = "be sure to change it"

[[judger]]
name = "http://127.0.0.1:8080"
type = "static"

[grpc]

[imgur]
client_id = "fffffffffffffff"
client_secret = "ffffffffffffffffffffffffffffffffffffffff"

```