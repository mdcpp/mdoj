# dev:
#     just dev-frontend & P1=$! & just dev-backend & P2=$! && wait P1 P2

dev-frontend:
    cd ./frontend && yarn dev

dev-backend:
    cd ./backend && cargo run

dev-sandbox:
    cd sandbox && sudo just test

clean:
    cd ./frontend && yarn cache clean
    cd ./backend && cargo clean
