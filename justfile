dev-frontend:
    cd frontend && just dev

dev-backend:
    cd backend && just dev

dev-judger:
    cd judger && cargo run

clean:
    cargo clean
