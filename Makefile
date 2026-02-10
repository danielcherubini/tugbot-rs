.PHONY: help setup test lint run build release check fmt clean db-up db-down migrate

# Default target - show help
help:
	@echo "tugbot-rs Makefile"
	@echo ""
	@echo "Available targets:"
	@echo "  make setup      - Set up development environment (start DB, run migrations)"
	@echo "  make test       - Run all tests"
	@echo "  make lint       - Run linting (clippy + format check)"
	@echo "  make run        - Run the bot locally"
	@echo "  make build      - Build the project (debug mode)"
	@echo "  make release    - Build the project (release mode)"
	@echo "  make check      - Quick compile check without building"
	@echo "  make fmt        - Format code with rustfmt"
	@echo "  make clean      - Clean build artifacts"
	@echo "  make db-up      - Start PostgreSQL container"
	@echo "  make db-down    - Stop PostgreSQL container"
	@echo "  make migrate    - Run database migrations"

# Set up development environment
setup: db-up migrate
	@echo "Development environment ready!"

# Run all tests
test:
	cargo test

# Run linting (clippy + format check)
lint:
	cargo clippy -- -D warnings
	cargo fmt --check

# Run the bot locally
run:
	cargo run

# Build the project (debug mode)
build:
	cargo build

# Build the project (release mode)
release:
	cargo build --release

# Quick compile check
check:
	cargo check

# Format code
fmt:
	cargo fmt

# Clean build artifacts
clean:
	cargo clean

# Start PostgreSQL container
db-up:
	docker-compose up -d
	@echo "Waiting for PostgreSQL to be ready..."
	@sleep 2

# Stop PostgreSQL container
db-down:
	docker-compose down

# Run database migrations
migrate:
	diesel migration run
