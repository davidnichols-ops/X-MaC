#!/bin/bash
set -e

# X-MaC LaunchAgent installer — runs a lightweight daily scan at 10:00 AM
# and notifies the user if reclaimable space exceeds a threshold.
# This is a foundation for scheduled/background automation.

LABEL="com.xmac.agent"
PLIST_DIR="$HOME/Library/LaunchAgents"
PLIST_PATH="$PLIST_DIR/$LABEL.plist"
LOG_DIR="$HOME/Library/Logs/X-MaC"
XMAC_BIN="$HOME/.local/bin/xmac"

usage() {
    echo "Usage: $0 [install|uninstall|status]"
    exit 1
}

install_agent() {
    if [ ! -x "$XMAC_BIN" ]; then
        echo "xmac binary not found at $XMAC_BIN. Install it first: cargo build --release && cp target/release/x-mac ~/.local/bin/xmac"
        exit 1
    fi

    mkdir -p "$PLIST_DIR"
    mkdir -p "$LOG_DIR"

    cat > "$PLIST_PATH" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>$LABEL</string>
    <key>ProgramArguments</key>
    <array>
        <string>$XMAC_BIN</string>
        <string>--format</string>
        <string>json</string>
        <string>--output</string>
        <string>$HOME/Library/Caches/com.xmac.gui/daily_scan.json</string>
        <string>quick</string>
    </array>
    <key>StartCalendarInterval</key>
    <dict>
        <key>Hour</key>
        <integer>10</integer>
        <key>Minute</key>
        <integer>0</integer>
    </dict>
    <key>StandardOutPath</key>
    <string>$LOG_DIR/agent.log</string>
    <key>StandardErrorPath</key>
    <string>$LOG_DIR/agent.error.log</string>
    <key>RunAtLoad</key>
    <false/>
</dict>
</plist>
PLIST

    launchctl unload "$PLIST_PATH" 2>/dev/null || true
    launchctl load "$PLIST_PATH"

    echo "Installed LaunchAgent: $PLIST_PATH"
    echo "Daily scan scheduled for 10:00 AM. Logs: $LOG_DIR"
}

uninstall_agent() {
    if [ -f "$PLIST_PATH" ]; then
        launchctl unload "$PLIST_PATH" 2>/dev/null || true
        rm "$PLIST_PATH"
        echo "Uninstalled LaunchAgent: $PLIST_PATH"
    else
        echo "No LaunchAgent installed at $PLIST_PATH"
    fi
}

status_agent() {
    if launchctl list | grep -q "$LABEL"; then
        echo "LaunchAgent $LABEL is loaded."
    else
        echo "LaunchAgent $LABEL is not loaded."
    fi
    if [ -f "$PLIST_PATH" ]; then
        echo "Plist exists: $PLIST_PATH"
    else
        echo "Plist does not exist."
    fi
}

case "${1:-install}" in
    install) install_agent ;;
    uninstall) uninstall_agent ;;
    status) status_agent ;;
    *) usage ;;
esac
