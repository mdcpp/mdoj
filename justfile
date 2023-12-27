# dev:
#     just dev-frontend & P1=$! & just dev-backend & P2=$! && wait P1 P2

dev-frontend:
    cd frontend && just run

dev-backend:
    cd backend && just run

dev-judger:
    cd judger && sudo just podman-run

prepare:
    # mkdir -p cert
    # openssl req -x509 -newkey rsa:4096 -keyout cert/key.pem -out cert/cert.pem -sha256 -days 3650 -nodes -subj "/C=XX/ST=StateName/L=CityName/O=CompanyName/OU=CompanySectionName/CN=localhost"

clean:
    cargo clean
