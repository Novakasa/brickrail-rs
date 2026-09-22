# Show the available recipes.
default:
    @just --list

# Run the combined client + simulation binary (single process).
run:
    cd brickrail-v2 && cargo run -p brickrail-client --bin brickrail-client

# Run the headless simulation server.
server:
    cd brickrail-v2 && cargo run -p brickrail-server

# Run the client-only binary (no simulation in-process).
client:
    cd brickrail-v2 && cargo run -p brickrail-client --bin client-only

# Dump the bevy plugin graph for every world into brickrail-v2/graphs/.
graphs:
    mkdir -p brickrail-v2/graphs
    cd brickrail-v2 && BEVY_PLUGIN_GRAPH=graphs/brickrail.mmd cargo run -p brickrail-server
    cd brickrail-v2 && BEVY_PLUGIN_GRAPH=graphs/brickrail.mmd cargo run -p brickrail-client --bin brickrail-client
    cd brickrail-v2 && BEVY_PLUGIN_GRAPH=graphs/brickrail.mmd cargo run -p brickrail-client --bin client-only

# Render Mermaid diagrams to SVG and open them. Defaults to every .mmd in the repo.
view *files:
    #!/usr/bin/env bash
    set -euo pipefail
    files=({{files}})
    if [ ${#files[@]} -eq 0 ]; then
        mapfile -t files < <(find . -name '*.mmd' -not -path '*/target/*' | sort)
    fi
    if [ ${#files[@]} -eq 0 ]; then
        echo "no .mmd files found; run \`just graphs\` first, or pass one explicitly" >&2
        exit 1
    fi
    out=$(mktemp -d -t view-graph-XXXXXX)
    for src in "${files[@]}"; do
        svg="$out/$(basename "$src" .mmd).svg"
        mmdc --input "$src" --output "$svg" --backgroundColor white
        echo "$svg"
        xdg-open "$svg" >/dev/null 2>&1 &
    done

# Regenerate the plugin graphs and open them.
graphs-view: graphs view

# Run the v2 test suite.
test:
    cd brickrail-v2 && cargo test --workspace

# Format the v2 workspace.
fmt:
    cd brickrail-v2 && cargo fmt --all
