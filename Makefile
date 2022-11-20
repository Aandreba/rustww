BROWSER=safari
SERVER_URL=http://localhost:8888

doc:
	RUSTFLAGS='-C target-feature=+atomics,+bulk-memory,+mutable-globals' cargo doc --open --target wasm32-unknown-unknown -Z build-std=std,panic_abort

test:
	RUSTFLAGS='-C target-feature=+atomics,+bulk-memory,+mutable-globals' wasm-pack build --target web --out-dir ../server/pkg tests -Z build-std=std,panic_abort
	@cp tests/index.html ./server/pkg/
	@cd server && cargo run & open http://localhost:3000