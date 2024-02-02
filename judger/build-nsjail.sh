#!/bin/sh
if [ ! -f nsjail-docker/output/*-linux-musl/nsjail-* ]; then
    echo nsjail not found, building one
    cd nsjail-docker && sudo make nsjail-3.1 > /dev/null
fi