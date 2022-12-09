SERVER_URL=http://localhost:8888
BROWSER=firefox

doc:
	cargo rustdoc --open --all-features --target wasm32-unknown-unknown -Z build-std=std,panic_abort -- --cfg docsrs --cfg web_sys_unstable_apis -C target-feature=+simd128,+atomics,+bulk-memory,+mutable-globals

test:
	RUSTFLAGS='--cfg web_sys_unstable_apis -C target-feature=+simd128,-atomics,-bulk-memory,-mutable-globals' wasm-pack test --$(BROWSER) --headless --features simd -Z build-std=std,panic_abort
	RUSTFLAGS='--cfg web_sys_unstable_apis -C target-feature=-simd128,-atomics,-bulk-memory,-mutable-globals' wasm-pack test --$(BROWSER) --headless -Z build-std=std,panic_abort

test_server:
	RUSTFLAGS='--cfg web_sys_unstable_apis -C target-feature=-simd128,+atomics,+bulk-memory,+mutable-globals' wasm-pack build --target web --out-dir ../server/pkg server_tests -Z build-std=std,panic_abort
	@cp tests/index.html ./server/pkg/
	@cd server && RUST_BACKTRACE=1 cargo run & open http://localhost:3000