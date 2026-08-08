.PHONY: generate rust-test build test verify signing-check signing-test install-local

generate:
	xcodegen generate

rust-test:
	cargo test --workspace

build: generate
	xcodebuild build -project AirControl.xcodeproj -scheme AirControl -destination 'platform=macOS,arch=arm64' -derivedDataPath .build/DerivedData CODE_SIGNING_ALLOWED=NO

test: generate
	xcodebuild test -project AirControl.xcodeproj -scheme AirControl -destination 'platform=macOS,arch=arm64' -derivedDataPath .build/DerivedData CODE_SIGNING_ALLOWED=NO

verify:
	./scripts/verify.sh

signing-check:
	./scripts/local-signing.sh check

signing-test:
	./integration_test/stable_local_signing.sh

install-local:
	./scripts/install-local.sh
