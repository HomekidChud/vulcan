#!/bin/sh
# scripts/e2e-discord.sh — end-to-end test for examples/discord.vulcan
#
# Boots the chat server, joins two scripted clients (alice and bob) over
# FIFOs, runs a conversation (channel chat, channel switch, direct messages,
# history), and verifies the conversation in the server log and in both
# clients' rendered output. The server runs on the C backend; the clients
# are exercised on C, JS and Python backends in turn.
#
# Usage: ./scripts/e2e-discord.sh [path-to-vulcan]
set -e
VULCAN="${1:-./target/release/vulcan}"
if [ ! -x "$VULCAN" ]; then echo "vulcan binary not found: $VULCAN (build with: cargo build --release)"; exit 1; fi

TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT
fails=0
check() { # check <name> <file> <pattern>
    if grep -q "$3" "$2"; then echo "[ok] $1"; else echo "[FAIL] $1 (missing '$3' in $2)"; fails=$((fails+1)); fi
}

# tiny helper: run a discord client with stdin from a fifo
start_client() { # start_client <name> <target(c|js|py)> <fifo>
    case "$2" in
        c)  "$DISCORD_BIN" client 127.0.0.1 "$PORT" "$1" < "$3" > "$TMP/$1.out" 2>&1 & ;;
        js) "$VULCAN" run --target js "$VULCAN_DIR/examples/discord.vulcan" client 127.0.0.1 "$PORT" "$1" < "$3" > "$TMP/$1.out" 2>&1 & ;;
        py) "$VULCAN" run --target py "$VULCAN_DIR/examples/discord.vulcan" client 127.0.0.1 "$PORT" "$1" < "$3" > "$TMP/$1.out" 2>&1 & ;;
    esac
}

VULCAN_DIR=$(cd "$(dirname "$0")/.." && pwd)
# build the example from a temp copy so artifacts never pollute the repo
cp "$VULCAN_DIR/examples/discord.vulcan" "$TMP/discord.vulcan"
"$VULCAN" build "$TMP/discord.vulcan" > /dev/null 2>&1
DISCORD_BIN="$TMP/discord.vbin"
if [ ! -x "$DISCORD_BIN" ]; then echo "[FAIL] could not build examples/discord.vulcan"; exit 1; fi

for TARGET in c js py; do
    echo "=== discord e2e: clients on $TARGET backend ==="
    rm -f "$TMP/server.log"
    "$DISCORD_BIN" server 0 > "$TMP/server.log" 2>&1 &
    SRV_PID=$!
    PORT=""
    for i in $(seq 1 50); do
        PORT=$(sed -n 's/^server: listening on port \([0-9]*\)$/\1/p' "$TMP/server.log" | head -1)
        [ -n "$PORT" ] && break
        sleep 0.1
    done
    if [ -z "$PORT" ]; then echo "[FAIL] server did not start"; cat "$TMP/server.log"; fails=$((fails+1)); continue; fi

    rm -f "$TMP/alice.in" "$TMP/bob.in"
    mkfifo "$TMP/alice.in" "$TMP/bob.in"
    start_client alice "$TARGET" "$TMP/alice.in"
    ALICE_PID=$!
    start_client bob "$TARGET" "$TMP/bob.in"
    BOB_PID=$!

    # alice's script (paced so the conversation interleaves realistically)
    (
        printf '/nick alice\n'
        sleep 0.4
        printf 'hello everyone\n'
        sleep 0.6
        printf '/join random\n'
        sleep 0.4
        printf 'yo from random\n'
        sleep 0.6
        printf '/msg bob psst bob dm me\n'
        sleep 1.2
        printf '/who\n'
        sleep 0.6
        printf '/quit\n'
    ) > "$TMP/alice.in" &
    WRITER_A=$!
    # bob's script
    (
        printf '/nick bob\n'
        sleep 0.8
        printf 'hi alice\n'
        sleep 1.0
        printf '/msg alice nice chat\n'
        sleep 1.2
        printf '/history\n'
        sleep 0.8
        printf '/quit\n'
    ) > "$TMP/bob.in" &
    WRITER_B=$!

    # wait for the scripted conversation to finish (not for the server!)
    wait $WRITER_A $WRITER_B
    # then give the clients a moment to drain and exit on their own
    for i in $(seq 1 100); do
        if ! kill -0 $ALICE_PID 2>/dev/null && ! kill -0 $BOB_PID 2>/dev/null; then break; fi
        sleep 0.1
    done
    kill $ALICE_PID $BOB_PID 2>/dev/null || true
    sleep 0.3
    kill $SRV_PID 2>/dev/null || true
    wait $SRV_PID 2>/dev/null || true

    # the conversation must be in the server log
    check "alice nick set (server)"        "$TMP/server.log" "server: guest[0-9]* -> alice"
    check "bob nick set (server)"           "$TMP/server.log" "server: guest[0-9]* -> bob"
    check "general broadcast (server)"      "$TMP/server.log" "\[#general\] alice: hello everyone"
    check "bob replies (server)"            "$TMP/server.log" "\[#general\] bob: hi alice"
    check "channel switch (server)"         "$TMP/server.log" "server: alice -> #random"
    check "random broadcast (server)"       "$TMP/server.log" "\[#random\] alice: yo from random"
    check "dm routed (server)"              "$TMP/server.log" "server: dm alice -> bob"
    check "dm routed (server 2)"            "$TMP/server.log" "server: dm bob -> alice"

    # and rendered correctly in both clients
    check "alice sees bob (client)"        "$TMP/alice.out" "\[#general\] bob: hi alice"
    check "alice sees dm (client)"          "$TMP/alice.out" "\[dm bob\] nice chat"
    check "alice joined #random (client)"  "$TMP/alice.out" "joined #random"
    check "bob sees alice (client)"         "$TMP/bob.out" "\[#general\] alice: hello everyone"
    check "bob sees dm (client)"            "$TMP/bob.out" "\[dm alice\] psst bob dm me"
    check "bob sees part notice (client)"   "$TMP/bob.out" "alice left #general"
    check "bob history replay (client)"    "$TMP/bob.out" "\[#general\] alice: hello everyone"
    check "bob history footer (client)"   "$TMP/bob.out" "end of history"
done

if [ $fails -gt 0 ]; then echo "$fails check(s) failed"; exit 1; fi
echo "discord e2e: all checks passed"
