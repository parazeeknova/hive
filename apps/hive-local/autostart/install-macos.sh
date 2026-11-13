#!/bin/sh
set -eu
executable=${1:-"$HOME/.cargo/bin/hive-local"}
configuration=${2:-"$HOME/Library/Application Support/hive-local/config.json"}
test -x "$executable" || { echo 'Install hive-local before installing the LaunchAgent.' >&2; exit 1; }
test -f "$configuration" || { echo 'Initialize hive-local before installing the LaunchAgent.' >&2; exit 1; }
# plistlib preserves spaces, ampersands and XML characters in user paths.
python3 - "$executable" "$configuration" <<'PY'
import os, pathlib, plistlib, sys
target = pathlib.Path.home() / 'Library/LaunchAgents/com.hive.local.plist'
target.parent.mkdir(parents=True, exist_ok=True)
logs = pathlib.Path.home() / 'Library/Logs/Hive Local'
logs.mkdir(parents=True, exist_ok=True)
with target.open('wb') as output:
    plistlib.dump({
        'Label': 'com.hive.local',
        'ProgramArguments': [sys.argv[1], 'run', '--config', sys.argv[2]],
        'RunAtLoad': True,
        'KeepAlive': True,
        'ThrottleInterval': 10,
        'EnvironmentVariables': {'PATH': str(pathlib.Path.home() / '.local/bin') + ':' + str(pathlib.Path.home() / '.cargo/bin') + ':/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin'},
        'StandardOutPath': str(logs / 'worker.log'),
        'StandardErrorPath': str(logs / 'worker-error.log'),
    }, output)
target.chmod(0o600)
PY
domain="gui/$(id -u)"
plist="$HOME/Library/LaunchAgents/com.hive.local.plist"
launchctl bootout "$domain" "$plist" 2>/dev/null || true
launchctl bootstrap "$domain" "$plist"
echo 'Hive Local starts now and at login. Remove it with launchctl bootout and delete the LaunchAgent plist.'
