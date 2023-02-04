build:
	cargo lambda build --release --arm64

start:
	cargo lambda start

test:
	cargo lambda invoke -F fixtures/hayden_dev.json
