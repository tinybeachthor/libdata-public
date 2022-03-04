build:
	cargo build --target wasm32-unknown-unknown
	wasm-bindgen \
		../target/wasm32-unknown-unknown/debug/libdata_wasm.wasm \
		--out-dir ./pkg
.PHONY: build

release:
	cargo build --release --target wasm32-unknown-unknown
	wasm-bindgen \
		../target/wasm32-unknown-unknown/debug/libdata_wasm.wasm \
		--out-dir ./pkg
	mv ./pkg/libdata_wasm_bg.wasm ./pkg/libdata_wasm_bg_orig.wasm
	wasm-opt -Os -o ./pkg/libdata_wasm_bg.wasm ./pkg/libdata_wasm_bg_orig.wasm
.PHONY: release

test:
	cargo test --target wasm32-unknown-unknown
.PHONY: test
