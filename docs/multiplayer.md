# Multiplayer

This implementation is based on upstream PR #2's merged commit
`cea2bc143eea25a0781bf91839ea0f2e97925c0d`. It lives on `feat/multiplayer`.
The frozen custom engine and the unfinished livery branch are excluded.

## Play with friends

Install the **same multiplayer build and converted assets** on every machine.
The protocol checks the physics source and selected track/car asset contents before admission.
Graphics, camera, audio and HUD choices remain local. Windows, macOS and Linux use the
same protocol; authoritative snapshots correct floating-point differences between platforms.

Build from the repository root:

```sh
cargo build --release --locked --workspace
```

Host, including the hosting player in the 16-player limit:

```sh
./target/release/f1-the-game --assets ./assets --car rb6 --host 7777 --name Alice --ai-cars 6
```

Join from another computer, replacing the address with the host's LAN or VPN IP:

```sh
./target/release/f1-the-game --assets ./assets --car rb6 --join 192.168.1.20:7777 --name Bob
```

On Windows use `target\release\f1-the-game.exe` with the same options. `--car gen22`
is also supported when the updated stock GEN22 assets, rig, teams and textures from
PR #2 are installed. The older RB22 asset directory cannot substitute for GEN22.

For the graphical flow, launch normally and choose **Multiplayer**, enter a unique
driver name, then **Host room** or **Join room**. Before creating the room, select
its circuit in Race setup and its car in Multiplayer. The owner can reorder the human grid and set laps,
an exact AI-car count (0–15, within the 16-car field limit), wing setup, tyres,
automatic gears, driving aid strength and DRS rules.
Aids can be locked for everyone or use each player's aids selected before joining.
DRS Anywhere remains an owner-controlled rule in both modes.

GEN22 players can select a stock team/driver or let the room assign a free driver.
The server rejects duplicate driver identities and sends one roster to every client.

Every player clicks **Ready**. The owner clicks **Start race**. In the lobby the owner can
set **AI cars** from 0 to the remaining available slots; that number is exact, rather than
automatically filling every spare slot. Readiness is cleared
when the owner changes rules. Everyone loads the same grid before the server runs
the countdown. No one may join after loading starts. Changing the circuit or car
requires creating a new room. Maximum field size is 16, including AI fill.

During the race, Esc opens the room menu while the shared race continues. Your car
brakes gently while the menu is open or the window is unfocused. R requests server
recovery; manual gear shifts and recovery have reliable sequence numbers. Local
camera keys, M and presentation settings stay local.

A disconnected guest's car becomes AI-controlled. Rejoining that running race is
not supported. If the owner leaves, everyone receives a terminated-session message.
At results, the owner can return everyone to the lobby and start another race.
Leaving a hosted room stops and joins its server worker.

## LAN and internet connectivity

Allow incoming **UDP port 7777** on the hosting computer. Manual joining remains
available without discovery or platform accounts. Both IPv4 and IPv6 numeric
addresses are accepted; IPv6 join addresses use `[address]:7777`.

For friends on separate networks, use a VPN connecting the computers, or forward
the UDP port on the host's router and share its reachable public address. There is
no NAT traversal, relay, matchmaking or room-code service in this implementation.
Do not expect two computers behind unrelated NATs to connect without one of these
routes. This is a friends-only, unauthenticated room service; it is not a ranked
anti-cheat or identity system.

A Linux, Windows or macOS machine can also run the headless server:

```sh
./target/release/racecraft-server --assets ./assets --bind 0.0.0.0:7777
```

The first admitted player owns a dedicated-server room. An optional `--settings
rules.json` reads a serialized `RaceSettings`. The server needs track collision,
centerline and car geometry assets, and hashes the same selected asset directories
as the clients. It does not link Bevy or require a display, sound device or GPU.
The same UDP process can run on a reachable VPS. No hosted deployment is included.

## Native packages

Run this on each destination OS:

```sh
python3 tools/package/build_multiplayer.py --assets ./assets --car rb6
```

The ZIP contains the game, dedicated server, selected assets, a launcher and these
instructions. Omitting `--assets` produces a binary-only package whose recipient
must install the matching assets separately. The workflow
`.github/workflows/multiplayer.yml` checks and builds all three operating systems;
its artifacts omit generated assets. A workflow definition is not evidence of a
successful remote build. Linux packages require system GPU/window/audio libraries;
this ZIP script does not replace the upstream Linux AppImage builder. macOS ZIPs
are native executables with a `.command` launcher, not signed/notarized apps.

## Simulation and network contracts

- The host owns one `Race` at `sim::PHYSICS_RATE` (360 Hz). Hosting players join
  through the same UDP client path as guests. Inputs target explicit server ticks.
- Clients keep a predicted full race, apply newer authoritative state, then replay
  pending inputs. The initial lead is 12 ticks (33 ms). Prediction stops at a
  90-tick lead during packet gaps; it cannot extrapolate indefinitely.
- Held input batches go at 60 Hz, repeating up to 36 recent physics samples.
  The server ignores past/duplicate ticks and rejects inputs more than 90 ticks
  into its future, non-finite controls and out-of-range controls.
- Shifts and recovery use a separate ordered reliable channel, monotonically
  increasing command IDs and authoritative snapshot acknowledgements. A repeated
  command ID cannot shift or recover the car twice. At most one command per car
  is consumed on each physics tick.
- Complete mutable snapshots include timing, race order, RNG, DRS, tyres,
  suspension and AI decision state. Immutable specs and large AI track planning
  caches remain local. Remote humans predict their last authoritative controls.
- Snapshots go at 30 Hz over Renet's unreliable fragmented channel. Snapshots
  are losslessly compressed; messages have a 64 KiB decoded size limit. Renet
  fragments below its UDP packet limit. Incomplete fragment storage is bounded
  at 8 MiB per channel, enough for three seconds of worst-size snapshots. Newer
  snapshots supersede older ones by server tick. Render corrections blend for
  100 ms without changing the corrected simulation state; recoveries snap.
- Lobby/load messages, numbered race events and final results use ordered reliable
  delivery. Events have bounded retention and are deduplicated before the HUD.
- Admission/load deadlines, bounded queues, per-client message budgets and an
  eight-second silence timeout keep abandoned sessions from living indefinitely.
  Unadmitted clients receive no race state. A 30-second load deadline ends the
  room if a connected participant cannot finish loading.
- The server bounds catch-up work at 36 physics steps per update and ends a room
  with a clear overload message if simulation debt exceeds half a second. It does
  not silently skip physics time to conceal sustained overload.

## Verification

```sh
cargo test --workspace --lib --bins
cargo test -p racecraft-net --test lifecycle -- --nocapture
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
cargo run --release -p racecraft-net --example soak -- ./assets 15
```

The network integration suite uses real loopback UDP sockets, 16 **scripted**
clients, deterministic 5% datagram loss, duplication every 31st datagram and
three-tick delay/reordering every seventh datagram. It checks fresh authority,
unique slot assignment, snapshot replay, disconnect takeover and malformed frames.
Additional lifecycle checks cover owner permissions, reliable command deduplication,
finish results and returning for another race. The soak uses the real converted
Spa/RB6 assets and a separately scheduled server, but has no renderer or human input.

Recorded checks and runtime limitations are reported with the delivery. Scripted
loopback capacity, a native build, and two people driving on separate computers
are different acceptance boundaries. Two-computer LAN driving, all-platform runtime
interop, long-distance latency and an externally deployed server require their own
validation; do not infer them from the integration suite.
