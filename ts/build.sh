#!/bin/bash
cd "$(dirname "$0")"
bun build src/main.ts --compile --outfile dist/rad
