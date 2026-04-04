set shell := ["cmd", "/c"]

clear := if os() == "windows" { "cls" } else { "clear" }

default:
    @just --list

project := "C:\\Users\\BraydenCarlson\\Documents\\code\\work\\stratusadv-portal"
interpreter := project + "\\.venv\\Scripts\\python.exe"

bindings_venv := "crates\\bindings\\.venv"
maturin := ".\\crates\\bindings\\.venv\\Scripts\\maturin.exe"

venv := ".venv"
python := ".\\.venv\\Scripts\\python.exe"

build:
    set "VIRTUAL_ENV=" && cargo run -p resolve -- --project {{project}} compile

test: ensure-venv
    cargo test -p compiler -p resolve

ensure-venv:
    @if not exist {{venv}} (echo Creating project venv... && uv venv --python {{interpreter}} {{venv}})

ensure-maturin:
    @if not exist {{bindings_venv}} (echo Creating bindings venv... && uv venv --python 3.12 {{bindings_venv}})
    @if not exist {{maturin}} (echo Installing maturin... && uv pip install --python {{bindings_venv}}\Scripts\python.exe maturin==1.12.6)

deploy: ensure-venv ensure-maturin
    {{clear}}
    cargo clean
    cargo build --release
    {{maturin}} build --profile release-python --manifest-path crates\bindings\Cargo.toml --interpreter {{python}}
    uv pip install --reinstall --no-cache --find-links target\wheels --python {{interpreter}} django-resolve
    set "VIRTUAL_ENV=" && .\target\release\resolve.exe --project {{project}} compile
