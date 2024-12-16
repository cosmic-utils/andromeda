name := 'andromeda'
appid := 'io.github.cosmic_utils.andromeda'
rootdir := ''
prefix := '/usr'
base-dir := absolute_path(clean(rootdir / prefix))
share-dir := base-dir / 'share'
bin-src := 'target' / 'release' / name
bin-dst := base-dir / 'bin' / name


clean:
    cargo clean

clean-vendor:
    rm -rf .cargo vendor vendor.tar

clean-dist: clean clean-vendor

build-debug *args:
    cargo build {{ args }}

build-release *args: (build-debug '--release' args)

build-vendored *args: vendor-extract (build-release '--frozen --offline' args)

check *args:
    cargo clippy --all-features {{ args }} -- -W clippy::pedantic

run *args:
    env RUST_LOG=andromeda=debug RUST_BACKTRACE=full cargo run --release {{ args }}

install:
    install -Dm0755 {{ bin-src }} {{ bin-dst }}

# Vendor dependencies locally
vendor:
    #!/usr/bin/env bash
    mkdir -p .cargo
    cargo vendor --sync Cargo.toml | head -n -1 > .cargo/config.toml
    echo 'directory = "vendor"' >> .cargo/config.toml
    echo >> .cargo/config.toml
    echo '[env]' >> .cargo/config.toml
    if [ -n "${SOURCE_DATE_EPOCH}" ]
    then
        source_date="$(date -d "@${SOURCE_DATE_EPOCH}" "+%Y-%m-%d")"
        echo "VERGEN_GIT_COMMIT_DATE = \"${source_date}\"" >> .cargo/config.toml
    fi
    if [ -n "${SOURCE_GIT_HASH}" ]
    then
        echo "VERGEN_GIT_SHA = \"${SOURCE_GIT_HASH}\"" >> .cargo/config.toml
    fi
    tar pcf vendor.tar .cargo vendor
    rm -rf .cargo vendor

# Extracts vendored dependencies
vendor-extract:
    rm -rf vendor
    tar pxf vendor.tar
