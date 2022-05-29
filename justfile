
dev-frontend:
    @cd ./frontend
    yarn dev

dev-backend:
    @cd ./backend
    cargo run

clean:
    cd ./frontend && yarn cache clean
    cd ./backend && cargo clean