# LogLens

A fast, local-first Rust CLI that analyzes application/server log files
and turns raw logs into useful diagnostic information.

## Status

Early development — v0.1.0. CLI skeleton complete, parser coming next.

## Install

    cargo install loglens

## Usage

    loglens analyze app.log
    loglens errors app.log
    loglens stats app.log
    loglens http app.log
    loglens version

