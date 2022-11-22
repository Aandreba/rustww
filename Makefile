BROWSER=safari
SERVER_URL=http://localhost:8888

doc:
	RUSTFLAGS='-C target-feature=+simd128,+atomics,+bulk-memory,+mutable-globals' cargo rustdoc --open --target wasm32-unknown-unknown -Z build-std=std,panic_abort -- --cfg docsrs

test:
	RUSTFLAGS='-C target-feature=+simd128,+atomics,+bulk-memory,+mutable-globals' wasm-pack build --target web --out-dir ../server/pkg tests -Z build-std=std,panic_abort
	@cp tests/index.html ./server/pkg/
	@cd server && RUST_BACKTRACE=1 cargo run & open http://localhost:3000