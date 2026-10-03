# Regenerate assets/demo.gif:
#   1. start a local Kafka and seed it:   see tapes/seed-demo.sh
#   2. cargo build --release && vhs tapes/demo.tape
# (set KITZ_DEMO_CONFIG to use a config other than tapes/demo.toml)
Output assets/demo.gif

Require kitz
Set Shell "bash"
Set FontSize 15
Set Width 1300
Set Height 720
Set Padding 12
Set Theme "Catppuccin Mocha"
Set TypingSpeed 60ms

Hide
Type `export PATH="$PWD/target/release:$PATH" D=$(mktemp -d) && cp "${KITZ_DEMO_CONFIG:-tapes/demo.toml}" "$D/kitz.toml" && cd "$D" && clear`
Enter
Show

Type "kitz stag"
Sleep 400ms
Enter
Sleep 3s

# Topics: everything about the selected topic loads by itself.
Down 3
Sleep 3.5s
Down
Sleep 1.5s
Up
Sleep 2s

# Latest messages, newest first.
Enter
Sleep 2.5s
Down 2
Sleep 1.5s
Escape
Sleep 800ms

# Filter.
Type "/ship"
Sleep 1.2s
Enter
Sleep 2s
Escape
Sleep 800ms

# Consumer groups and their lag.
Tab
Sleep 2.5s
Down
Sleep 2s
Down
Sleep 2s

# Switch environment - prod is marked.
Type "2"
Sleep 3s
Type "q"
Sleep 500ms
