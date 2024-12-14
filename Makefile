
.PHONY dev
make dev
	cargo build dev

.PHONY release
make release
	cargo build release