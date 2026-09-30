.PHONY: build up down clean check experiment local-build

EXP ?= ch09-default-route

build:
	docker compose build

up:
	docker compose up -d

down:
	docker compose down

clean: down
	rm -f captures/*.pcap reports/*.json reports/*.md

check:
	@docker --version
	@docker compose version

local-build:
	cargo build

experiment: build
	@if command -v cargo >/dev/null 2>&1; then \
		cargo run -- $(EXP); \
	else \
		docker run --rm \
			-v "$(PWD):/work" \
			-w /work rust:1.82-bookworm \
			bash -c 'cargo build'; \
		./target/debug/netlab $(EXP); \
	fi
