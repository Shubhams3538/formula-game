# Implement playable multiplayer in F1 Racecraft

Build multiplayer in the actual Bevy game so two people on separate computers can host, join, drive, race, and finish together. Target up to 16 players, with a playable two-computer LAN session as the first milestone.

This handoff was prepared on 2026-11-02. It requests implementation by the recipient; the preparation of this document did not change game or networking code.

## Start from main after PR #2 merges

- Base the multiplayer implementation on the latest `origin/main` after [PR #2](https://github.com/jR4dh3y/formula-game/pull/2), `feat: prepare 2.0 gameplay with a shared stock GEN22 car`, has merged. Its source branch is `feat/gen22-car-and-cockpit`.
- PR #2 was still open when this instruction was recorded. Verify its merge state and fetch the latest remote main before creating the implementation branch. If it remains open, record the dependency and leave implementation pending until it merges.
- Create a separate multiplayer branch and worktree from that main revision. Record the exact base SHA. Do not use the current working checkout as the implementation base.
- Do not base this work on, merge, or cherry-pick `feat/gen22-liveries`. That work is unfinished and is outside this delivery. Use the stock GEN22 assets provided by main with PR #2.
- Preserve the existing checkout and its unrelated changes, including the untracked `:7777` file. Copy this handoff into the implementation checkout without importing other changes.

## Preserve these constraints

- Bevy is the only active engine. Rendering, input, UI, audio, and cameras belong in `game/`. Physics and race rules belong in `sim/`.
- Do not edit or build `archive/custom-engine/` for this work, or restore its engine dependencies.
- Run Cargo from the root of the checkout being tested. Keep asset conversion and packaging in root `tools/`.
- Preserve unrelated work. Do not reset, delete, or overwrite unrelated changes. Check the exact diff before staging anything.
- Do not commit or push without Radhey's authorization.
- Read the applicable `AGENTS.md` files. Use `hit-every-surface` for the implementation and `finish-the-work` for broad or interrupted work.
- Verify behavior at the real boundary. A build, localhost exchange, or synthetic benchmark does not prove playable LAN racing or 16-player capacity.

## Define the networking contracts

- Give inputs explicit per-client sequence numbers and server acknowledgements. Define how input samples map to physics ticks. Reconcile local simulation against authoritative state, then replay pending inputs using compatible simulation behavior. Keep render smoothing separate from simulation correction.
- Drive remote-car interpolation with an advancing estimate of server time. Reject stale snapshots before they affect state. Bound extrapolation during packet gaps.
- Separate held controls from one-shot commands. A sampled manual shift or recovery request must have one intended effect under retransmission, duplication, and packet loss.
- Give race events stable IDs and bounded retention. Deliver finish results reliably and avoid repeated notices.
- Model loading, readiness, countdown, racing, finish, and return explicitly. Define who can start, when joining closes, and what happens when a participant leaves.
- Make join retries bounded and idempotent. Handle rejection, timeout, server silence, leave, and subsequent joining without stale session state.
- Validate session identity, packet sizes, names, input age, finite numeric values, control ranges, and traffic rates. Send race state only to admitted clients.
- Bound queues and handshake lifetimes. Handle slow readers, backpressure, transport failures, and worker cleanup.
- Use `sim::PHYSICS_RATE` as the timestep source of truth. Define bounded catch-up and overload behavior, and compare elapsed simulation time with wall time.
- Measure encoded packet sizes at the target field size and race duration. Establish a UDP packet budget and fragmentation policy before capacity claims.

## Follow the actual game paths

Recheck these code paths on the selected main revision before editing. The navigation notes below came from a working checkout and are not proof that every symbol exists unchanged on the required base. Do not import missing behavior from the livery branch to match this document.

Trace ownership of the local `Race`, the `Session` resource in `game/src/main.rs`, and the `FixedUpdate` path that supplies human input to `Race::step`. Online clients must stop independently deciding authoritative race state.

Audit and update these paths together:

- `game/src/assets.rs::load_race` loads the converted track, car geometry, entrants, and assists. Extract only the headless setup needed by the server. Do not make a server require a renderer or visual GLB loading. Identify matching track, physics, car, and session configuration during join, and reject incompatible content with a useful message.
- `game/src/menu.rs` creates and replaces sessions, mutates assists, starts races, and manages pause. Add host, direct join, loading/readiness, errors, leave, and return behavior. Server policy must govern race configuration and permitted assist changes.
- `game/src/controls.rs` directly calls `session.race.recover(player)`. Recovery must become a validated server command online. Keep local camera and presentation actions local.
- Pause and focus loss currently set `session.paused`, and `step_race` stops. An online pause menu cannot stop the shared race. Define safe input behavior when unfocused, paused, disconnected, or loading.
- `game/src/pose.rs`, car updates in `game/src/assets.rs`, and `game/src/camera.rs` depend on local simulation poses. Supply consistent predicted and interpolated poses at display rate, including wheels, suspension, and recovery discontinuities.
- `game/src/hud.rs`, `game/src/hud/`, `game/src/wheel_dash.rs`, and `game/src/audio/` consume car and timing state. Trace the fields they actually read. Replicate the authoritative data they need, including sectors, deltas, warnings, race phase, finish state, and ordered events. A position packet is insufficient.
- `game/src/options.rs`, `game/src/preferences.rs`, tests, README launch commands, and root `tools/package/` need review for host/join options, persisted settings, included binaries, and platform dependencies.

Keep offline racing working through its existing local simulation path. Camera choices, graphics, audio volume, and other local preferences remain client concerns. Account systems, matchmaking, host migration, spectators, and archive-engine integration are outside the first playable LAN milestone.

## Build in verifiable stages

1. **Choose authority and measure the host budget.** Recommend one authoritative `Race` in a headless server, with the host game joining through the same client contract. Keep physics at 360 Hz. Start with 60 Hz input delivery and 30 Hz snapshots as tunable targets. Measure a real converted circuit with a 16-car field, serialization, networking, and the host renderer active. Record frame times, tick deadline misses, and bandwidth. Hosting overhead needs measurement under real race conditions.
2. **Deliver two playable Bevy clients over direct-IP LAN.** Use UDP first unless a measured constraint changes that choice. Load the same real track and cars, assign distinct players, wait for readiness, start together, and drive both cars using real controls. Implement compatible local prediction and remote interpolation, then prove shared contact, lap timing, and finish results. Minimal host/join UI is part of this stage.
3. **Complete session and failure behavior.** Handle full sessions, incompatible versions/content, duplicate joins, lost joins, input loss, stale packets, explicit leave, crashes, server disappearance, and a second session. Choose and document what happens to abandoned cars and whether rejoining a running race is allowed. Host migration is not required; clear termination on host loss is acceptable for the first release.
4. **Scale and package.** Validate up to 16 human connections, including the hosting player. Exercise real-track races, prolonged event traffic, loss, jitter, and slow clients. Build runnable host/client packages for each claimed OS. Add LAN discovery as a convenience after direct-IP joining works; discovery failure must leave manual joining usable.
5. **Add hosted online play when in scope.** Use the same session/message semantics over WebSockets if the selected host requires them. Verify `wss://`, current hosting ingress rules, environment port binding, health checks, restarts, and timeout behavior on the deployed target. Select hosting based on its current transport support and operating limits. Do not claim this stage complete from localhost plain WebSockets.

Before implementation, record which stages the delivery covers. For a full LAN-and-online delivery, complete all five. Finish stage 2 as the first reviewable milestone.

## Accept the result at the player boundary

For the first playable milestone, record a bounded session on two physical computers, with one hosting and playing. Use the same build and converted circuit assets. Show that each person controls only their car, sees the other moving, starts the same race, and receives matching authoritative outcomes. Exercise contact, braking, one manual gearshift, DRS rules, recovery, focus loss, and pause. Then finish or deliberately end the session, return to the menu, and join another session.

Add focused automated checks for slot assignment, input ordering and deduplication, command consumption, prediction replay and convergence, advancing interpolation, event delivery, result agreement, and cleanup. Inject packet loss, duplication, reordering, delay, and server loss. Record the injection settings and observed correction behavior. Do not replace driving review with a test that only checks receipt of a snapshot.

For capacity claims, record 16 connected humans or clearly label scripted clients. Report the circuit, build, OS, host hardware, network topology, race duration, packet sizes, tick delays, frame times, and disconnects. Scripted capacity tests and two-machine driving prove different things.

Run focused tests first, then these repository checks from the implementation checkout root:

```bash
cargo test --workspace --lib --bins
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
```

Also run the new networking integration tests explicitly. `--lib --bins` does not select integration test targets. Follow README instructions for applicable Vulkan renderer checks and state any unavailable runtime or platform validation. Test direct-IP connectivity and discovery separately on the actual LAN.

Return the implementation with the exact checkout and revision, changes made, checks that passed, runtime evidence, and important untested flows. Leave incomplete stages explicitly marked unfinished.
