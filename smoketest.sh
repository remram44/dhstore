#!/bin/sh

set -eux

r(){
    CMD="$1"
    shift
    target/debug/dhstore -v -v "$CMD" -d store "$@"
}

r init
r verify

r add src/
r verify
