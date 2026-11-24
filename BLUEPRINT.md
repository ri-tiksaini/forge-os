# FORGE OS - MASTER ENGINEERING BLUEPRINT
Document ID: FORGE-BP-001 | Revision 1.1 | Date 2026-11-24
Status: AUTHORITATIVE. If code and this document disagree, this document wins until a
formal revision is filed (see 0.4).
Audience: an engineer who has never spoken to the product owner. Follow it literally.

Working product name used throughout: Forge OS.
User-visible brand strings live in branding/brand.toml (see 0.3). Internal technical
identifiers (crate names, paths, D-Bus/Polkit namespaces, binary magic) remain stable.

================================================================================
0. HOW TO USE THIS DOCUMENT
================================================================================

0.0 REVISION 1.1 CHANGELOG (engineering audit fixes over v1.0)
  - Base & compositors (2.2, 6.2, 6.4, 7.3): aligned to real Debian 13 "trixie"
    packages (labwc 0.8.3 on Tier 0, sway 1.10.1 on Tier 1/2); deferred Hyprland
    (trixie-backports only) to optional post-v1 catalog; dropped impossible
    labwc tiling parity requirement.
  - Packages & hardware (6.2): enabled Debian non-free-firmware for real laptop
    Wi-Fi/GPU microcode; added missing greetd, plymouth, bootloaders, fs tools,
    ca-certificates, sudo, and pinned mise binary; removed redundant seatd,
    second terminal (alacritty), and Qt file manager (pcmanfm-qt -> thunar).
  - Parking engine (5.6): removed non-existent systemd StopIdleSec= directive;
    specified systemd-socket-proxyd (--exit-idle-time) + StopWhenUnneeded=yes
    for daemons without native sd_listen_fds (PostgreSQL, MySQL, Redis).
  - Catalog format (5.1): removed compile-time phf (incompatible with HTTPS OTA
    updates) and redundant crc32; standardized on binary search + Ed25519 sig.
  - Security & rollback (2.5, 6.5.2, 6.5.3): mandated SO_PEERPIDFD / SO_PEERCRED
    for Polkit authorization; scoped Tier 0/1 apt rollback honestly as best-effort
    purge + dpkg recovery + config restore (atomic rollback on Tier 2 Btrfs/v2).
  - Shell stack & backups (3, 6.7, 6.10): unified forge-shell on GTK4 +
    libgtk4-layer-shell0 (avoiding custom text/IME/AT-SPI engine and Mesa GL
    RSS blowup); scoped ~/.config backups to an allowlist excluding app caches.
  - CI & schedule (0.3, 8.1, 9.4, 10): scoped 0.3 to user-visible strings;
    demoted bit-for-bit ISO hash (G10) to release-branch audit; adjusted staffing.

0.1 READING ORDER (do not skip)
  1  Goals, non-goals, acceptance numbers   -> what "done" means
  2  Architecture + process model           -> the shape of the system
  3  Language policy                        -> what to write where
  4  Repository layout                      -> create the tree exactly
  5  Data structures & algorithms           -> the performance contract
  6  Component build specs                  -> the actual work, in order
  7  UI specification                       -> pixels, timings, states
  8  Build, packaging, release              -> how it ships
  9  Test & verification matrix             -> how you prove it
  10 Milestones & definition of done        -> when you stop

0.2 RULE OF LITERAL COMPLIANCE
Every spec block has: PURPOSE / INPUTS / OUTPUTS / ALGORITHM / COMPLEXITY BUDGET /
FAILURE MODES / ACCEPTANCE TEST. Implement all seven. A section with no passing
acceptance test may not be marked complete.

0.3 NAMING DISCIPLINE
User-visible brand strings (product name, display titles, URLs, colors, fonts,
wallpaper, Plymouth theme, /etc/os-release display fields) live ONLY in
branding/brand.toml. No UI or user-facing source file may hardcode the product
name; read it via forge-brand generated constants. Internal technical identifiers
(crate names, /run/forge/forged.sock, /var/lib/forge/, org.forge.* Polkit/D-Bus
namespaces, FORGE_TIER, binary magic "FGCT") are intentionally fixed to avoid
breaking IPC, paths, and upgrades across rebrands.

0.4 CHANGE CONTROL
Any deviation requires docs/adr/NNNN-title.md: Context / Decision / Consequences /
Measured impact on the 1.3 budgets. No ADR, no merge.

0.5 NON-NEGOTIABLES (violating any one fails the project)
N1 Idle memory at first desktop <= budget in 1.3 for the detected tier.
N2 Nothing the user installs auto-starts without explicit consent.
N3 Every task has a GUI path AND a CLI path. Neither may be missing.
N4 No capability is removed from the system; it is deferred or parked.
N5 Tier 0 must reach a usable desktop with GPU acceleration disabled.
N6 Upstream lineage stays discoverable (ID_LIKE in os-release, upstream repos reachable).
N7 Every destructive action has a tested, single-command / single-click recovery path
   (atomic Btrfs snapshot on Tier 2; journaled undo + config restore on Tier 0/1).
N8 Cold start of any resident app <= 400 ms on Tier 1 reference hardware.

================================================================================
1. GOALS, NON-GOALS, ACCEPTANCE NUMBERS
================================================================================

1.1 PRODUCT THESIS (memorize; every decision serves this)
"Everything you need is already here. Everything you don't need is one word away -
and never runs until you ask."

Target user: anyone who writes code, first-year student through staff engineer.
Low floor (GUI, mouse, plain language), high ceiling (CLI, tiling, modal editors).
Hardware range: 2011-era 4 GB HDD laptop through 2026 NVMe workstation.

1.2 EXPLICIT NON-GOALS (do NOT build these; they cause scope death)
- Not a general-purpose consumer distro. No office suite, media center, games preinstalled.
- Not a from-scratch kernel. We ship a tuned upstream LTS kernel.
- Not a new package format. We orchestrate existing formats (deb / flatpak / OCI / mise).
- Not an immutable-only system in v1. The bootc path is v2 (8.6).
- Not a custom GUI toolkit. GTK4 + libadwaita (+ gtk4-layer-shell) for all Forge UI.
- Not an AI-first OS. A local LLM is a parked optional service, never a dependency.
- No telemetry, no accounts, no cloud requirement, no phone-home. Ever.

1.3 HARD PERFORMANCE BUDGETS (measured on reference hardware, 9.1)

Metric                                Tier 0 Legacy   Tier 1 Standard  Tier 2 Modern
------------------------------------  --------------  ---------------  --------------
Idle RSS, all user processes          <= 420 MiB      <= 720 MiB       <= 1100 MiB
Idle PSS (shared-aware, the real #)   <= 300 MiB      <= 560 MiB       <= 900 MiB
Boot: firmware-exit -> greeter        <= 22 s (HDD)   <= 9 s (SSD)     <= 6 s (NVMe)
Greeter -> usable desktop             <= 3.0 s        <= 1.5 s         <= 1.0 s
Launcher open -> first result         <= 60 ms        <= 35 ms         <= 25 ms
Launcher keystroke -> repaint         <= 16 ms        <= 16 ms         <= 8 ms
App cold start (terminal)             <= 700 ms       <= 400 ms        <= 250 ms
Compositor frame budget               16.6 ms 60fps   16.6 ms          8.3 ms 120fps
Installed base image (compressed)     <= 1.8 GiB      same image       same image
Installed base on disk                <= 4.8 GiB      <= 5.8 GiB       <= 6.8 GiB
Background wakeups/sec at idle        <= 4            <= 4             <= 6
dev search over 5000 catalog items    <= 20 ms        <= 12 ms         <= 8 ms

Measurement method is normative (9.2). Do not estimate these numbers. Measure them.

1.4 FUNCTIONAL ACCEPTANCE (the demo script that proves the product)
A tester who has never used Linux must, unaided, in under 15 minutes:
F1 Boot the ISO, connect Wi-Fi from a graphical list, create an account.
F2 Land on a desktop showing only the Forge brand (no upstream name visible in the UI).
F3 Open a store, install a code editor with one click, see its RAM/disk cost first.
F4 Create a "Web project" from a wizard; end with a folder, deps installed, editor open.
F5 Run the project and see it in the browser.
F6 Install PostgreSQL; verify it consumes 0 RAM until first use and parks after idle.
F7 Break something intentionally; restore with one click / one command.
F8 Repeat F3-F6 using only the terminal, with dev commands.

================================================================================
2. SYSTEM ARCHITECTURE
================================================================================

2.1 BASE DECISION (LOCKED for v1)
Base: Debian 13 "trixie" stable (main + non-free-firmware), kernel 6.12 LTS series.
Debian 13 was released 2025-08-09 and ships the 6.12 LTS kernel [3][4]; trixie remains
the current stable branch [2]. Rationale: unbreakable base for beginners, best
old-hardware coverage, lowest maintenance load for a small team. Freshness comes from
Flatpak (GUI apps) and mise (language runtimes), not from the base [33][34].
v2 option: Fedora bootc image-based delivery for atomic rollback [9][13]. Do not start
it in v1, but do not design against it (8.6).
Kernel policy: full driver and firmware coverage retained (N5). We tune scheduling and
memory, not drivers.

2.2 LAYER DIAGRAM (bottom to top)
 L0  Hardware
 L1  Linux 6.12 LTS (preempt=full, BFQ/kyber, zram, MGLRU) + non-free-firmware
 L2  Debian trixie minimal rootfs (debootstrap --variant=minbase)
 L3  systemd (masked-by-default) + PipeWire + NetworkManager(iwd) + logind
 L4  Compositor: labwc 0.8.3 (T0) | sway 1.10.1 (T1 & T2) <- ONE config generator
 L5  forge-shell      launcher + status pill + notifications   [Rust + GTK4 layer-shell]
 L6  forge-settings   single control panel                     [Rust + GTK4/libadwaita]
 L7  forged           catalog / transaction daemon             [Rust, socket-activated]
 L8  dev              thin CLI client of forged                [Rust, same workspace]
 L9  Resident apps    foot, firefox-esr, thunar, zathura, xarchiver (unified GTK/Wayland)
 L10 Declared catalog everything else, zero footprint until requested

Compositor versions pinned from Debian 13 "trixie" main (both share wlroots 0.18):
  - labwc 0.8.3-1 (Tier 0: lightweight stacking/floating, pixman software fallback) [18]
  - sway 1.10.1-2 (Tier 1 & Tier 2: hybrid tiling + floating, native IPC) [20]
Hyprland (only in trixie-backports at 0.55.2+ds-1~bpo13+1) [25] is deferred to an
opt-in post-v1 catalog package to keep the v1 base 100% trixie-main with zero backport
maintenance burden.

2.3 PROCESS MODEL - EXACTLY which processes exist at idle
This table is the contract for N1. Anything not listed must not be running.

Process                Lang   Activation        Idle PSS target  Notes
---------------------  -----  ----------------  ---------------  -------------------------
systemd (PID1)         C      always            ~12 MiB          masked units per 6.2
systemd-journald       C      always            ~8 MiB           volatile, capped 32 MiB
systemd-logind         C      always            ~4 MiB           replaces seatd
dbus-broker            C      always            ~3 MiB           preferred over dbus-daemon
NetworkManager         C      always            ~14 MiB          iwd backend
iwd                    C      always            ~5 MiB           replaces wpa_supplicant
pipewire               C      socket            ~10 MiB          starts on first audio client
wireplumber            C      socket            ~8 MiB
compositor             C      always            40 / 70 MiB      labwc (T0) / sway (T1,T2)
forge-shell            Rust   always            <= 36 MiB        GTK4+layer-shell, 1 proc
greetd                 Rust   boot only         exits after login
forged                 Rust   socket-activated  0 at idle        exits after 90 s idle
systemd-socket-proxyd  C      socket-activated  0 at idle        per proxied parked svc
xdg-desktop-portal     C      dbus-activated    0 at idle        + wlr/gtk backend on demand
polkitd                C      dbus-activated    0 at idle
earlyoom               C      always            ~1 MiB           kills compilers, not desktop

NOT RUNNING at idle (parked but available): bluetoothd, cupsd, avahi-daemon,
ModemManager, sshd, podman services, postgresql, ollama. NEVER INSTALLED: tracker,
baloo, packagekit (forged replaces it), any background update checker.

2.4 IPC MAP (who talks to whom, and how)
 dev CLI        --unix /run/forge/forged.sock-->  forged   (JSON-RPC 2.0, newline framed)
 forge-settings --same socket------------------>  forged
 forge-shell    --same socket------------------>  forged   (search/install only)
 forge-shell    --wlr-layer-shell (GTK4)------->  compositor
 forged         --dbus system bus-------------->  systemd  (StartUnit/StopUnit for parking)
 forged         --exec------------------------->  apt-get / flatpak / mise (streamed)
 sway           --sway IPC socket-------------->  forge-shell (workspace/window events)

RULE: one socket, one protocol. Do not add a second IPC mechanism. JSON-RPC chosen over
D-Bus for our own API because it is debuggable with socat and testable without a bus.
D-Bus is used only where the platform requires it (systemd, portals, notifications).
Note: labwc (Tier 0) does not expose sway IPC; forge-shell uses wlr-foreign-toplevel /
ext-workspace Wayland protocols when running under labwc.

2.5 TRUST BOUNDARIES
- forged runs as root (needs apt/systemd). Read-only API for any local user; mutating API
  gated by Polkit action org.forge.catalog.install.
- Caller authentication (MANDATORY): forged MUST obtain caller identity via SO_PEERPIDFD
  (Linux 6.5+, preferred) or SO_PEERCRED + /proc/&lt;pid&gt;/stat start-time verification on the
  Unix domain socket, and pass that verified subject to Polkit to prevent PID-reuse
  TOCTOU races.
- forge-shell, forge-settings, dev run as the user. They NEVER exec apt directly.
- Catalog Ed25519 signature verified before any load or transaction (6.5.4). Unsigned = refuse.
- No network listener except the unix socket. No TCP ports open by default (ss -tulpn empty).

================================================================================
3. LANGUAGE POLICY (what is written in what, and why)
================================================================================

Component                Language         Justification
-----------------------  ---------------  ------------------------------------------------
forged + dev CLI         Rust             one static binary, no runtime, ~4 MiB RSS,
                                          safe concurrency for transactions
forge-shell              Rust + GTK4      gtk4-rs + libgtk4-layer-shell0; shares GTK4/
                                          Pango/Cairo pages with forge-settings; native
                                          HarfBuzz/IME/CJK/HiDPI/AT-SPI without a custom
                                          renderer; cairo software renderer on Tier 0
forge-settings           Rust + GTK4      gtk4-rs + libadwaita gives modern look for free
forge-notify (in shell)  Rust             same binary, feature-gated
installer / first-boot   Rust + GTK4      reuses forged transaction engine, no second impl
tier detection           Rust             parses /proc, /sys, drm; deterministic, testable
config generation        Rust             one generator emits labwc/sway/GTK configs
build / CI glue          Python 3         BUILD-TIME ONLY, never shipped in the image
system integration glue  POSIX sh         only where 5 lines beat a binary (initramfs hooks)
kernel tuning            sysctl + units   no code
catalog data             TOML             human-editable, diff-friendly, compiled once
theming                  CSS (GTK)        libadwaita variables only

BANNED in shipped artifacts: Python scripts, Node.js, Electron, Qt runtimes, bashisms in
/bin/sh, any interpreted runtime as a dependency of a resident component. Reason:
interpreter startup (20-120 ms) and RSS (15-60 MiB) blow the 1.3 budgets. Exception:
python3-minimal exists on disk as a system dependency for user development, but no Forge
component may import it.

RUST POLICY
- Edition 2021. MSRV pinned exactly in rust-toolchain.toml; bump only via ADR.
- #![forbid(unsafe_code)] everywhere except thin FFI wrappers (e.g., SO_PEERPIDFD
  getsockopt), which use #![deny(unsafe_op_in_unsafe_fn)] inside src/ffi/.
- Release profile: opt-level=3, lto="fat", codegen-units=1, panic="abort", strip="symbols".
  Keep debuginfo in a separate -dbgsym package for crash triage.
- Async: tokio ONLY in forged (concurrent subprocess streams). GTK4 apps (forge-shell,
  forge-settings) use GLib MainContext; no second async runtime.
- Errors: thiserror for libraries. anyhow forbidden in shipped paths; we need typed
  errors mapped to JSON-RPC codes.
- Logging: tracing, compile-time max level info in release; --verbose flips an AtomicU8 filter.
- No source file exceeds 600 lines. Split by responsibility. No utils.rs anywhere.

================================================================================
4. REPOSITORY LAYOUT (create exactly this tree)
================================================================================

forge-os/
|-- BLUEPRINT.md                  this document, always at repo root
|-- Cargo.toml                    workspace manifest
|-- rust-toolchain.toml           pinned toolchain
|-- Makefile                      thin wrapper over cargo + xtask
|-- xtask/src/main.rs             cargo xtask image|iso|test|bench|brand|catalog
|-- branding/
|   |-- brand.toml                SINGLE SOURCE OF TRUTH: name, colors, urls
|   |-- logo.svg
|   |-- wallpaper/                three weights, one per tier
|   `-- plymouth/                 boot splash theme
|-- crates/
|   |-- forge-brand/              build.rs reads brand.toml -> emits constants
|   |-- forge-proto/              JSON-RPC types shared by daemon and clients
|   |   `-- src/{lib,request,response,error}.rs
|   |-- forge-catalog/            model, index, search, rank, load, sig  (NO side effects)
|   |-- forge-txn/                plan, apply, rollback, journal
|   |   `-- src/backends/{apt,flatpak,mise,oci,systemd}.rs
|   |-- forge-tier/               probe, score, profile
|   |-- forge-conf/               emit_labwc, emit_sway, emit_gtk, keymap
|   |-- forged/                   main, server, dispatch, park, policy, progress, ffi/
|   |-- dev-cli/                  main + cmd/{add,remove,search,new,list,repair,doctor}
|   |-- forge-shell/              main, launcher/, pill/, notify/, ipc/
|   |-- forge-settings/           main + pages/{appearance,services,hardware,projects,about}
|   `-- forge-installer/          main, steps/, partition, users, network
|-- data/
|   |-- catalog/                  THE PRODUCT: one TOML per app
|   |   |-- _schema.json          JSON Schema, validated in CI
|   |   |-- editors/ languages/ databases/ containers/ tools/ stacks/
|   |-- profiles/                 tier0.toml tier1.toml tier2.toml cpu_years.toml
|   |-- keymap.toml               ONE keymap, emitted to labwc + sway
|   |-- theme/                    gtk.css, colors, icons
|   `-- systemd/                  units, masks, presets, timers, proxies/
|-- image/
|   |-- rootfs.mk                 debootstrap recipe (main + non-free-firmware)
|   |-- packages.list             exact apt set, version-pinned
|   |-- flatpaks.list             preinstalled flatpak refs, commit-pinned
|   |-- overlays/                 files copied verbatim into the image
|   `-- hooks/                    initramfs + post-install hooks (sh)
|-- tests/
|   |-- unit/                     in-crate #[cfg(test)] + fixtures/
|   |-- integration/              daemon+CLI over a real socket, temp rootfs
|   |-- vm/                       boots the image in QEMU, asserts 1.3 budgets
|   `-- hardware/                 manual checklists for real machines
|-- bench/                        idle_ram.sh boot_time.sh launcher_latency.rs search_bench.rs
`-- docs/
    |-- adr/                      decision records
    |-- user/                     end-user docs site source
    `-- upstream.md               honest lineage + where to get upstream help (N6)

================================================================================
5. DATA STRUCTURES & ALGORITHMS (the performance contract)
================================================================================
This section is why the OS feels instant on a 2011 laptop. Implement exactly as written.

5.1 CATALOG STORAGE - parse once, mmap forever
Problem: 500-5000 catalog entries. Naive approach parses TOML on every search: 200 ms+.
Solution: compile to a binary index at BUILD time; sign with Ed25519; load with mmap;
zero-copy access.

Build step (cargo xtask catalog):
 1 Read every data/catalog/**/*.toml, validate against _schema.json.
 2 Intern repeated strings (categories, backends, licenses) into a deduplicated pool.
 3 Emit catalog.bin + detached Ed25519 signature catalog.bin.sig with this layout:

 Header (64 bytes, cache-line aligned)
   magic[4] = "FGCT"      version:u32     entry_count:u32   flags:u32
   off_entries:u64        off_strings:u64 off_trigram:u64   off_deps:u64
   file_len:u64           reserved[8]     (total 64 bytes)

 Entry (fixed 96 bytes, sorted ascending by id_hash)
   id_hash:u64            FxHash of lowercase id
   name_off:u32           name_len:u16      category:u16
   backend:u8             activation:u8     tier_min:u8    autostart:u8
   disk_mb:u32            ram_idle_mb:u32   ram_active_mb:u32
   popularity:u16         flags:u16
   keywords_off:u32       keywords_len:u16  deps_off:u32   deps_len:u16
   desc_off:u32           desc_len:u16      payload_off:u32 payload_len:u32

 String pool: length-prefixed UTF-8, deduplicated, 4-byte aligned.
 Trigram index: see 5.2.
 Dependency edges: CSR (compressed sparse row) = offsets[] + targets[].

Complexity: load O(1) after Ed25519 verification (mmap, no parse). Lookup by id O(log n)
binary search on sorted id_hash (~13 comparisons for 5000 entries; compatible with runtime
HTTPS catalog updates—do NOT use compile-time phf). Memory = file size, shared across all
clients through the page cache: three processes reading it cost ONE physical copy.
Budget: 5000 entries -> ~1.2 MiB file. Resident cost ~0 because it is file-backed/evictable.
Failure modes: bad magic, truncated file_len, or Ed25519 signature mismatch -> refuse to
load, fall back to /var/lib/forge/catalog.prev.bin (also signature-verified), log once,
surface in dev doctor.

5.2 SEARCH - trigram inverted index with bounded fuzzy fallback
Requirement: substring and typo-tolerant search, <= 20 ms over 5000 entries, <= 8 ms typical.

Structure:
 - Extract all 3-grams from name + id + keywords (lowercased, ASCII-folded).
 - Pack each trigram into a u32 (3 bytes).
 - trigram_table: sorted array of (key:u32, offset:u32, len:u32)
 - postings: flat array of entry indices, contiguous per trigram.

Query algorithm (pseudocode, implement literally):
   fn search(q):
     q = normalize(q)                        # lowercase, fold accents, collapse spaces
     if q.len < 3:
        candidates = prefix_range(q)         # binary search on sorted names, O(log n + k)
     else:
        grams = trigrams(q)                  # O(|q|)
        lists = [postings(g) for g in grams] # O(log T) each, binary search
        sort lists by length ascending
        candidates = intersect_sorted(lists) # galloping search, O(m log(n/m))
     score each candidate                    # 5.3
     partial_sort top K (K = 12)             # O(c log K) with a BinaryHeap
     return top K

Fallback when candidates is empty: bounded Levenshtein, max distance 2, bit-parallel
Myers algorithm over the name array, O(n * ceil(m/64)). Runs only on a miss.
Why not tantivy/lucene: index size and RSS. A trigram index over 5000 short strings is
~200 KiB and answers in microseconds. Do not add a dependency here.
Acceptance: criterion bench, cold and warm, must meet the 1.3 search budget; results
identical between cold and warm runs (determinism test).

5.3 RANKING - deterministic, cheap, explainable, integer-only
   score = 100 * exact_id_match
         +  60 * name_prefix_match
         +  30 * keyword_exact
         +  15 * (matched_grams / total_grams)     # coverage, computed as fixed-point
         +  10 * popularity_normalized             # curated 0..1, NOT usage telemetry
         +   8 * is_installed
         +   5 * tier_fit
         -  20 * requires_heavy_runtime_on_tier0
All integer arithmetic (coverage as x1000 fixed point). No floats in the hot path.
Ties broken by lexicographic id so results are stable across runs (testability).

5.4 LAUNCHER HOT PATH - zero allocation in search query path
 - Preallocated bump arena reset per query; no heap allocation inside forge-catalog::search.
 - Result list is a fixed [Option&lt;EntryRef&gt;; 12] on the stack; GTK ListModel updates rows
   in place (diffing by EntryId) without rebuilding widgets.
 - Query text buffer reserved at startup; never reallocated below 256 chars.
 - Debounce: NONE for the local catalog (<8 ms). 120 ms debounce ONLY for the optional
   file-search provider, which runs on a worker thread cancellable via AtomicBool.
Acceptance: heap profile of search path shows zero allocations; p95 keystroke->frame <= 16 ms.

5.5 DEPENDENCY RESOLUTION FOR TRANSACTIONS
Catalog deps form a DAG. CI validates acyclicity with Kahn's algorithm and fails on a cycle.
Install planning:
   plan(requested_ids):
     needed    = topo_sort(transitive_closure(requested))   # O(V+E) over CSR adjacency
     missing   = needed - installed_set                     # roaring bitmap difference
     cost      = sum(disk_mb); peak_ram = max(ram_active_mb)
     steps     = group_by_backend(missing)                  # batch apt into ONE call
     return Plan{steps, cost, rollback_token}
Batching matters: one apt-get install a b c is roughly 3x faster than three invocations
because dpkg trigger processing dominates. NEVER loop apt per package.
installed_set = roaring bitmap over entry indices: O(1) membership, KB-scale serialization,
SIMD-fast set ops. Rebuilt from dpkg/flatpak state at daemon start, then kept incrementally.

5.6 PARKING ENGINE (the "hide, don't deprive" mechanism)
Every catalog entry with activation = "on-demand" uses one of two normative patterns
(systemd has no StopIdleSec= directive; do not invent unit keys):

 Pattern A - Native sd_listen_fds services (e.g., forged, podman, cups):
   - Ship a systemd .socket unit (or .path unit). The .service is NOT enabled and NOT
     wanted by any target.
   - The daemon accepts the systemd-passed FD and exits cleanly after 90 s idle.

 Pattern B - Non-socket-activated daemons (e.g., PostgreSQL, MariaDB, Redis):
   - Disable the upstream .service so it never starts at boot; configure it to listen on
     a private internal socket/loopback port (e.g., /run/postgresql-internal/ or 127.0.0.1:5433)
     and set StopWhenUnneeded=yes on the upstream .service.
   - Ship a public proxy .socket on the standard port/path (e.g., 127.0.0.1:5432 and
     /run/postgresql/.s.PGSQL.5432) paired with a proxy .service running:
       /usr/lib/systemd/systemd-socket-proxyd --exit-idle-time=90s &lt;private-target&gt;
     with Requires=&lt;daemon&gt;.service and After=&lt;daemon&gt;.service.
   - Flow: psql connects -> systemd activates proxy + PostgreSQL -> proxy forwards traffic
     -> 90 s after last connection closes, proxy exits -> StopWhenUnneeded=yes stops
     PostgreSQL cleanly. Idle memory returns to baseline.
   - Caveat: if a developer's app holds an open idle connection pool, the proxy stays active
     until the pool disconnects or the user runs `dev off <id>` (or toggles it in Settings).

Acceptance: after `dev add postgres`, `systemctl is-active postgresql` == inactive and the
idle PSS delta is <= 5 MiB; running `psql` transparently starts it; 90 s after closing
`psql`, `systemctl is-active postgresql` returns to inactive.

5.7 TIER DETECTION - deterministic scoring, no ML
Inputs, read once and cached to /run/forge/tier.json:
   mem_total_kb       /proc/meminfo
   cpu_model, flags   /proc/cpuinfo (sse4_2, avx2, core count)
   cpu_year_proxy     curated model-string lookup, data/profiles/cpu_years.toml
   disk_rotational    /sys/block/*/queue/rotational
   disk_model         /sys/block/*/device/model
   gpu_vendor, driver /sys/class/drm/card*/device/{vendor,driver}
   gles_version       EGL query; on failure assume GLES2
   display_count      drm connectors

   score = w_mem*f(mem) + w_cpu*f(year) + w_disk*f(rot) + w_gpu*f(gles)
   weights: w_mem=35, w_cpu=25, w_disk=20, w_gpu=20  (each f normalized 0..1)
   thresholds: score < 35 -> T0 ; 35..70 -> T1 ; > 70 -> T2

 HARD OVERRIDES (applied after scoring; they always win):
   mem_total < 3.5 GiB              -> force T0
   rotational == 1                  -> cap at T1
   gles < 3.0 OR software rasterizer-> force T0
   no working DRM driver            -> force T0 + nomodeset boot entry

Deterministic means unit-testable with fixture files. Ship 40 fixtures of real machines in
tests/unit/fixtures/ and assert the tier for each. This is how N5 is guaranteed.
Public API:
   pub struct TierReport { tier: Tier, score: u8, reasons: Vec&lt;Reason&gt;, raw: Probes }
   pub fn detect() -> TierReport;                      // reads the system
   pub fn detect_from(p: &Probes) -> TierReport;       // pure, for tests
Env override FORGE_TIER=0|1|2 for testing only; logged loudly when active.

5.8 MEMORY STRATEGY
 - zram: zstd compression, size = min(50% RAM, 4 GiB).
 - vm.swappiness = 100 when zram is the ONLY swap. With compressed RAM swap, swapping
   early is correct; the classic "swappiness=10" advice applies to DISK swap only.
   If a disk swap exists: swappiness = 10.
 - vm.page-cluster = 0 for zram (read one page, not eight; random access pattern).
 - vm.watermark_scale_factor = 125 on Tier 0: start reclaim earlier, avoid allocation stalls.
 - MGLRU: enable /sys/kernel/mm/lru_gen/enabled if present (6.12 exposes it).
 - THP = madvise only. Never "always" (compaction causes latency spikes).
 - journald: Storage=volatile, RuntimeMaxUse=32M. Persistent logs opt-in on Tier 2.
 - /tmp on tmpfs, size 25% RAM, nosuid,nodev.
 - earlyoom: -m 5 -s 10 --avoid '^(labwc|sway|forge-shell|firefox-esr)$'
   --prefer '^(cc1plus|rustc|node|ld)$'. Kill the compiler, not the desktop.

5.9 I/O STRATEGY
 - HDD (rotational=1): BFQ, nr_requests=64, read_ahead_kb=256.
 - SSD/NVMe: none (mq) or kyber; read_ahead_kb=128.
 - ionice class 2 niceness 7 for every bulk operation forged launches (apt, flatpak).
   Interactive apps stay class 2 nice 0, so typing never stutters during an install.
 - ext4 mount options: noatime,lazytime,commit=30. Tier 2 may use Btrfs for snapshots.
 - NEVER disable journaling on the root filesystem. Data loss risk outweighs the gain.

5.10 STARTUP STRATEGY
 - Parallel systemd startup. Forbid After=network-online.target on anything user-facing.
 - Compositor starts before NetworkManager finishes; the Wi-Fi applet fills in async.
 - Tier 0 prefetch: a oneshot unit during greeter display running under ionice -c 3 (idle)
   reads only the compositor + forge-shell + foot binaries (capped at 48 MiB) so it never
   evicts active boot pages or delays greeter input.
 - initramfs-tools MODULES=dep: only storage, filesystem, input modules. Smaller, faster.
 - No splash longer than the actual boot. Plymouth exits the moment the greeter is ready.

================================================================================
6. COMPONENT BUILD SPECIFICATIONS
================================================================================
Build in this exact order. Each has an acceptance test. Do not start N+1 before N passes.

--------------------------------------------------------------------------------
6.1 forge-brand                                                     [Week 1, day 1]
--------------------------------------------------------------------------------
PURPOSE   single source of user-visible brand truth.
INPUT     branding/brand.toml
OUTPUT    generated Rust constants; /etc/os-release; desktop files; Plymouth theme.
ALGORITHM build.rs parses TOML, emits const items, writes dist files to target/brand/.
BUDGET    O(size of brand.toml), build-time only.
FAILURE   missing key -> build fails naming the exact key.
ACCEPTANCE grep for upstream brand names in UI strings returns nothing brand-related;
          editing brand.toml + rebuild changes every visible UI string and /etc/os-release.

Canonical brand.toml:
   name         = "Forge OS"
   id           = "forge"
   id_like      = "debian"        # N6 honest lineage, machine-readable
   version      = "1.0"
   codename     = "ember"
   pretty_name  = "Forge OS 1.0 (Developer Edition)"
   home_url     = "https://forge.example.org"
   support_url  = "https://forge.example.org/help"
   bug_url      = "https://forge.example.org/bugs"
   upstream_doc = "https://wiki.debian.org"   # N6 users can still get help
   accent       = "#E8703A"
   bg           = "#0E1013"
   fg           = "#E6E8EB"
   radius_px    = 10
   font_ui      = "Inter"
   font_mono    = "JetBrains Mono"

Generated /etc/os-release must contain NAME, VERSION, ID, ID_LIKE, PRETTY_NAME,
HOME_URL, SUPPORT_URL, BUG_REPORT_URL, LOGO=forge-logo. Kernel LOCALVERSION="-forge".

--------------------------------------------------------------------------------
6.2 Image base + systemd policy                                     [Week 1-3]
--------------------------------------------------------------------------------
PURPOSE smallest rootfs that still boots and connects Wi-Fi on real hardware.
RECIPE  image/rootfs.mk
   debootstrap --variant=minbase --components=main,non-free-firmware \
       --include=<packages.list> trixie $ROOTFS http://deb.debian.org/debian
   apt configured with APT::Install-Recommends "false" globally inside the image.
   mise installed via xtask rootfs as a pinned, SHA-256-verified static binary at
   /usr/local/bin/mise (not packaged in Debian trixie).

packages.list (EXACT set, all in Debian trixie main/non-free-firmware, version-pinned):
   linux-image-amd64 linux-firmware-free firmware-iwlwifi firmware-misc-nonfree
   firmware-amd-graphics firmware-intel-graphics
   systemd systemd-sysv systemd-boot grub-pc-bin efibootmgr plymouth greetd
   e2fsprogs btrfs-progs dosfstools sudo ca-certificates dbus-broker
   network-manager iwd pipewire wireplumber pipewire-pulse
   polkitd xdg-desktop-portal xdg-desktop-portal-wlr xdg-desktop-portal-gtk
   labwc sway swaybg swaylock libgtk4-layer-shell0
   foot firefox-esr thunar zathura xarchiver
   git lazygit make gcc pkg-config python3-minimal
   flatpak podman slirp4netns fuse-overlayfs
   fonts-inter fonts-jetbrains-mono adwaita-icon-theme
   grim slurp wl-clipboard brightnessctl playerctl
   zram-tools earlyoom

MASKED UNITS (ship as symlinks to /dev/null in image/overlays):
   bluetooth.service cups.service cups-browsed.service avahi-daemon.service
   ModemManager.service apt-daily.timer apt-daily-upgrade.timer man-db.timer
TIMERS WE KEEP: fstrim.timer (weekly, SSD only), forge-config-backup.timer
   (daily, ~200 ms), systemd-timesyncd (needed, ~2 MiB).

ACCEPTANCE VM with 2 GiB RAM and emulated rotational disk boots to a labwc desktop;
   systemd-analyze <= 22 s; systemctl list-units --state=running matches 2.3 exactly;
   Wi-Fi hardware enumerates on ThinkPad X220/T480.

--------------------------------------------------------------------------------
6.3 forge-tier                                                      [Week 4]
--------------------------------------------------------------------------------
As 5.7. CLI surface: dev doctor --tier prints the report with plain-language reasons.
ACCEPTANCE 40 fixtures produce expected tiers; FORGE_TIER override works and is logged.

--------------------------------------------------------------------------------
6.4 forge-conf                                                      [Week 5-6]
--------------------------------------------------------------------------------
PURPOSE ONE keymap/theme/profile -> TWO compositor configs + GTK4 theme. Never hand-edit.
INPUT   data/keymap.toml, data/profiles/tierN.toml, brand.toml
OUTPUT  ~/.config/labwc/{rc.xml,autostart}, ~/.config/sway/config,
        ~/.config/gtk-4.0/gtk.css
ALGORITHM template-free emission: build a small AST of Bind{mods,key,action}, serialize
        per backend. Guarantees identical semantics for all shared actions.
INVARIANT core navigation and launch shortcuts behave identically on all tiers (N3).
        Workspace tiling toggle (Super+T) is active on Tier 1/2 (sway); on Tier 0
        (labwc, stacking-only), Super+T snaps the focused window to half-screen and
        displays a one-time hint that full tiling requires Tier 1+.

Canonical keymap (LOCKED):
   Super             launcher toggle          Super+Return    terminal (foot)
   Super+E           file manager (thunar)    Super+B         browser (firefox-esr)
   Super+Q           close window             Super+F         fullscreen
   Super+Shift+F     float toggle             Super+T         tiling toggle (T1/2) / snap (T0)
   Super+Tab         next window              Super+1..9      workspace 1..9
   Super+Shift+1..9  move window to N         Super+Left/Right focus left/right
   Super+Shift+S     screenshot region        Super+L         lock (swaylock)
   Super+Shift+E     logout menu              Ctrl+Alt+F2     rescue TTY (always works)

ACCEPTANCE golden-file tests byte-match committed goldens; a bind-parity test asserts
   every core action exists in both generated outputs.

--------------------------------------------------------------------------------
6.5 forge-catalog + forge-txn + forged                              [Week 7-12]
--------------------------------------------------------------------------------
THIS IS THE PRODUCT. Everything else is packaging around it.

6.5.1 Catalog entry schema (data/catalog/_schema.json, enforced in CI)
   id            = "postgres"
   name          = "PostgreSQL"
   category      = "database"
   summary       = "Relational database server"
   keywords      = ["sql","db","psql","relational"]
   backend       = "apt"          # apt | flatpak | mise | oci | stack
   packages      = ["postgresql"]
   activation    = "on-demand"    # resident | on-demand | manual
   parking_mode  = "socket-proxy" # none | native-socket | socket-proxy | path
   autostart     = false          # MUST be false unless category = core
   disk_mb       = 320
   ram_idle_mb   = 0
   ram_active_mb = 180
   tier_min      = 0
   license       = "PostgreSQL"
   popularity    = 0.8
   depends       = []
   conflicts     = []
   post_install  = ["create-dev-role","install-proxy-units","register-launcher"]
   gui_entry     = false
   cli_entry     = ["psql"]
   beginner_note = "Starts only when a project needs it."
   heavy_warning = ""

 Validation rules (CI fails on violation):
   - autostart=true only if category="core" AND an ADR exists.
   - ram_idle_mb > 0 requires activation="resident" plus a justification field.
   - activation="on-demand" requires parking_mode != "none".
   - every entry discoverable in the launcher via gui_entry or cli_entry.
   - stack entries reference only existing ids (checked at build).

6.5.2 JSON-RPC API (forge-proto), frozen v1
   catalog.search  {query, limit?, category?}          -> [EntrySummary]
   catalog.get     {id}                                -> EntryFull
   catalog.cost    {ids[]}                             -> {disk_mb, peak_ram_mb, warnings[]}
   txn.plan        {ids[], remove?}                    -> {token, steps[], cost}
   txn.apply       {token}                             -> stream of Progress notifications
   txn.rollback    {token}                             -> Result
   system.state    {}                                  -> {tier, installed[], parked[], ram}
   service.enable  {id}                                -> Result (polkit)
   service.disable {id}                                -> Result (polkit)
   project.create  {stack, path, name}                 -> {path, editor_cmd}
   system.repair   {level: configs|packages|full}      -> stream Progress

 Transport: unix socket, newline-delimited JSON. Caller verified via SO_PEERPIDFD /
 SO_PEERCRED before Polkit check (2.5). Progress = notifications with method txn.progress
 carrying {token, step, total, percent, line}.
 Error codes: standard -32xxx plus application codes in error.rs:
   1001 NotFound          1002 Unsigned            1003 InsufficientDisk
   1004 TierWarningUnacknowledged                  1005 BackendFailed
   1006 RollbackUnavailable                        1007 PermissionDenied
   1008 Conflict

6.5.3 Transaction engine - journaled, resumable, recoverable
   Plan     pure function, no side effects; ordered Steps + cost + uuid token.
   Apply    on Tier 2 (Btrfs), take a pre-txn snapshot first. Write-ahead journal to
            /var/lib/forge/journal/&lt;token&gt;.json BEFORE each step.
            Step = {backend, argv, undo_argv, idempotency_key}.
            Execute with ionice nice 7; stream stdout into progress notifications.
            On failure: stop, mark journal FAILED, offer recovery/rollback.
   Rollback on Tier 2: revert to pre-txn Btrfs snapshot (atomic).
            on Tier 0/1 (ext4): best-effort recovery—first run `dpkg --configure -a` and
            `apt-get -f install` if an apt step was interrupted, then replay undo_argv in
            reverse for completed steps, and restore affected configs from backup. Never
            claim "nothing was changed" unless verification confirms clean state (7.5).
   Crash safety: on daemon start scan the journal dir; IN_PROGRESS found -> offer
            resume-or-rollback in the UI and in dev doctor.

 Backends:
   apt.rs      ONE batched apt-get -y --no-install-recommends install ...
               undo = apt-get -y purge &lt;exact newly-added set&gt;. Record that exact set
               before execution. NEVER blanket autoremove.
   flatpak.rs  flatpak install --system -y --noninteractive &lt;ref&gt;, commit-pinned.
   mise.rs     mise use -g tool@ver (global) or mise use tool@ver inside a project dir.
   oci.rs      podman pull; parked via podman.socket, never a running daemon at idle.
   systemd.rs  enable/disable/mask units and proxy sockets; backs service.enable/disable.

6.5.4 Signature verification
   catalog.bin signed with Ed25519 (catalog.bin.sig); public key compiled into the binary.
   Updates fetched over HTTPS from brand.home_url, verified into a temp file, then swapped
   atomically with rename(2). Unsigned -> error 1002. No GPG keyring management for users.

6.5.5 Daemon lifecycle
   Socket-activated by forged.socket. On start: verify + mmap index, build installed_set,
   scan journals, serve. Exit after 90 s with no client and no active transaction;
   systemd restarts it on the next connection. A package manager that costs 0 RAM at idle.

ACCEPTANCE
   - dev search postg returns PostgreSQL within the 1.3 budget (CI bench gate).
   - Installing postgres leaves postgresql.service inactive; idle PSS delta <= 5 MiB;
     connecting via psql starts it and idle disconnect parks it after 90 s.
   - Power-cut mid-transaction then reboot offers recovery; recovery restores clean dpkg
     and catalog state.
   - ss -tulpn shows no TCP listeners.

--------------------------------------------------------------------------------
6.6 dev CLI                                            [Week 10-12, parallel]
--------------------------------------------------------------------------------
Thin client. NO business logic. Every command maps 1:1 to an RPC.
   dev add <id...>         catalog.cost + txn.plan + confirm + txn.apply
   dev remove <id...>      txn.plan(remove) + apply
   dev search &lt;query&gt;      catalog.search; table: id, name, disk, ram, status
   dev list [--parked]     system.state
   dev new &lt;stack&gt; [name]  project.create
   dev on &lt;id&gt; / off &lt;id&gt;  service.enable / disable
   dev doctor              tier report, journal check, disk space, broken deps
   dev repair [--level]    system.repair
   dev update              curated system bundle transaction (8.3)
   dev help                plain language, one screen, no jargon

UX RULES
   - Every success line prints the equivalent GUI location ("Also in Settings > Software").
   - Confirmations show cost. --yes skips for scripting.
   - Exit codes: 0 ok, 1 generic, 2 usage, 3 permission, 4 insufficient resources, 5 backend.
   - TTY-aware output: pretty table on a TTY, --json for machines, never colorize when piped.
ACCEPTANCE F8 passes using only these commands.

--------------------------------------------------------------------------------
6.7 forge-shell                                        [Week 13-18]
--------------------------------------------------------------------------------
ONE process, three roles (launcher, pill, notifications) built on Rust + GTK4 +
libgtk4-layer-shell0 (shares GTK4/Pango/Cairo pages with forge-settings; gives HarfBuzz
text shaping, CJK/emoji, IME, fractional scaling, and AT-SPI accessibility for free).
Rendering: GSK Cairo renderer on Tier 0 (CPU, mandatory for N5); default GSK GL/NGL on
Tier 1/2.
Phased validation note: during M0-M2, stand in fuzzel + waybar + mako to prove the base
image, catalog, and parking engine before replacing them with forge-shell in M3.

Launcher state machine:
   Hidden --Super--> Opening(120ms fade + 8px rise) --> Ready
   Ready --keystroke--> Querying(<8ms) --> Ready
   Ready --Enter--> Executing --> Hidden(80ms fade)
   Ready --Esc--> Hidden
   Ready --idle 30s--> Hidden

Providers (each returns <= 5 results, merged by rank):
   1 Apps        desktop entries cached at startup, inotify-refreshed
   2 Catalog     mmap index, 5.2
   3 Actions     lock, logout, settings pages, tier switch, performance mode
   4 Projects    ~/.local/share/forge/projects.json
   5 Files       optional worker thread, fd-style walk, early cancel, 120 ms debounce
   6 Shell hist  read once, ranked recency * frequency

Progressive disclosure (the teaching trick): each row shows the GUI label on line 1 and
the equivalent command in dim monospace on line 2. Users learn the CLI passively.

Pill: 220x28 px, top-center, layer-shell exclusive zone 0 (overlays, reserves no space).
Contents: workspace dots | git branch if cwd is a repo | CPU sparkline | RAM % | clock.
Expands to 520x180 on hover/click showing parked services and active tasks.
Refresh: 1 Hz normally, 4 Hz during a transaction, 0 Hz when locked.
Cost ceiling: < 0.4% CPU on Tier 0. Verify with perf stat.

Notifications: implements org.freedesktop.Notifications inside the same GTK4 binary;
hides overlay surfaces when queue is empty.

ACCEPTANCE
   - bench/launcher_latency.rs p95 keystroke->repaint <= 16 ms on the Tier 0 VM.
   - forge-shell PSS <= 36 MiB after 1 hour uptime (no memory growth over 1000 opens).
   - Works under WLR_RENDERER=pixman and GSK_RENDERER=cairo (software). Mandatory test.

--------------------------------------------------------------------------------
6.8 forge-settings                                     [Week 17-20]
--------------------------------------------------------------------------------
GTK4 + libadwaita, Rust. ONE window, six pages. Replaces every scattered config tool.
 1 Appearance  accent, dark/light, animation level (Off/Subtle/Full), font scale, wallpaper.
               Changing animation level regenerates configs via forge-conf and reloads the
               compositor live. No logout.
 2 Software    the Store: same search engine as the launcher, categories, install buttons,
               cost preview dialog, installed list with remove.
 3 Services    the parking panel: every parked service with toggle, active RAM, last used.
               Subtitle: "Off means installed but silent."
 4 Hardware    tier report in plain language, Performance Mode toggle (drops one tier
               instantly), power profile, disk health.
 5 Projects    created projects, open/remove, stack versions, New Project wizard.
 6 About       brand, version, upstream lineage link (N6), docs, backup/restore,
               "Repair this system" with three levels explained in plain words.

RULES no page takes >150 ms to open (lazy-load on first visit). No modal dialogs except
destructive confirmations. Every destructive dialog names the exact undo action.
ACCEPTANCE F3, F6, F7 pass using only this app.

--------------------------------------------------------------------------------
6.9 forge-installer + first-boot                       [Week 19-22]
--------------------------------------------------------------------------------
One binary, two modes: --live-install (from ISO) and --first-boot (OOBE).
Exactly five screens, no more:
 1 Language + keyboard (auto-detected defaults, one click to accept)
 2 Network (Wi-Fi list via NM/iwd; Skip allowed - offline install must work)
 3 Account (username, password, strength meter; autologin default OFF)
 4 Path (checkboxes Web / Systems / Data / Minimal). This ONLY writes a deferred install
   queue. It does NOT install during OOBE. The desktop appears immediately, then a
   notification offers "Set up your Web tools (450 MB) - Start / Later".
   Rationale: never make a beginner stare at a progress bar for ten minutes.
 5 Done -> desktop.

Partitioning (live-install): guided whole-disk, ext4 on T0/T1, Btrfs + subvolumes on T2.
Manual mode behind "Advanced". zram always; disk swap only if RAM < 4 GiB AND hibernation
requested. Bootloader: systemd-boot on UEFI, GRUB on BIOS (old machines need it, N5).
Plymouth from branding. cmdline: quiet splash loglevel=3 vt.global_cursor_default=0.
ACCEPTANCE full install on a BIOS-emulated QEMU VM with 2 GiB RAM completes; F1/F2 pass.

--------------------------------------------------------------------------------
6.10 Safety net                                        [Week 21-22]
--------------------------------------------------------------------------------
 - forge-config-backup.timer: daily tar.zst into /var/lib/forge/backups/, keep 7, cap
   200 MiB, prune oldest. Target ~200 ms/day.
   BACKUP SCOPE (explicit allowlist—NEVER archive raw ~/.config wholesale):
     /etc (excluding /etc/shadow-, transient caches)
     ~/.config/{labwc,sway,gtk-4.0,foot,forge,git,mise}/
     ~/.bashrc, ~/.profile, ~/.gitconfig
   Explicitly excluded: browser profiles, Electron/VS Code caches, *.sqlite*, Cache/.
 - Tier 2: Btrfs snapshot before each transaction (<= 1 s) for atomic rollback.
 - Tier 0/1: NO filesystem snapshots (disk and I/O cost). Instead: transaction journal
   recovery (6.5.3) + allowlisted config backups. Sufficient and honest.
 - Rescue boot entry "Forge OS - Rescue" -> multi-user.target, no compositor, prints a
   six-line plain-language banner including dev repair and dev doctor.
 - dev repair --level configs   restore allowlisted dotfiles from backup
              --level packages  verify dpkg state (dpkg --configure -a), reinstall broken
                                resident packages
              --level full      both + regenerate compositor configs
ACCEPTANCE F7 passes on all three tiers; backup archive stays < 25 MiB on a machine with
   Firefox and VS Code installed.

================================================================================
7. UI SPECIFICATION (pixels, timings, states)
================================================================================

7.1 DESIGN TOKENS (from brand.toml, emitted to CSS and compositor configs)
   bg        #0E1013    surface  #16191E    surface2 #1E2229
   fg        #E6E8EB    muted    #9AA3AD    border   #2A2F37
   accent    #E8703A    ok #4CC38A    warn #E5B567    err #E5484D
   radius    10px base; windows 12px; pill 14px; buttons 8px
   spacing   4 / 8 / 12 / 16 / 24 / 32  (4px grid; nothing off-grid)
   font-ui   Inter 14px / line-height 20px; headings 20/28 semibold
   font-mono JetBrains Mono 13px / line-height 19px
   elevation shadow 0 8 24 rgba(0,0,0,.45)   Tier 2 only
   blur      none in v1 (solid surfaces across all tiers; avoids wlroots blur patches)
Contrast: all text >= 4.5:1 against its background (WCAG AA). CI checks every token pair.
Accessibility is not optional and is not a later phase.

7.2 MOTION SPEC (tier-scaled, one source of truth)
Transition          Tier 0        Tier 1                Tier 2
------------------  ------------  --------------------  ---------------------------------
Launcher open       90 ms fade    120 ms fade+8px rise  160 ms fade+12px rise, ease-out-expo
Launcher close      60 ms fade    80 ms fade            100 ms fade+4px fall
Window open         instant       instant (sway)        instant (sway)
Window close        instant       instant (sway)        instant (sway)
Workspace switch    instant       instant (sway)        instant (sway)
Pill expand         80 ms width   140 ms width          200 ms critically-damped spring
Easing: entrances cubic-bezier(0.05, 0.9, 0.1, 1.0); exits cubic-bezier(0.4, 0.0, 1, 1).
RULE motion never blocks input. Animation is decorative; the action commits at t=0.
RULE if frame time > 20 ms for 30 consecutive frames, auto-drop one animation level and
     notify once: "Reduced effects to keep things smooth." Never ask. Expose the override
     in Settings > Appearance.

7.3 DESKTOP COMPOSITION
 - No dock. No taskbar. No desktop icons. Wallpaper (swaybg) + pill only.
 - Wallpaper: Tier 0 static PNG (~200 KB dark gradient); Tier 1/2 static WebP.
 - Window management:
     Tier 0 (labwc): floating with smart cascade placement (24px offset); Super+T snaps
                     focused window half-screen.
     Tier 1/2 (sway): tiling by default for power users OR floating toggle per window
                      (Super+Shift+F) and workspace layout toggle (Super+T); guided mode
                      explains window controls on first launch.
 - Workspaces: 9, unnamed by default, user-renamable. Occupancy dots in the pill.
   NO AI auto-naming (cut in review: gimmick, costs RAM, confuses users).

7.4 LAUNCHER VISUAL ANATOMY
   +----------------------------------------------------------+  width 640, radius 12
   |  [search icon]  type to search apps, tools, files...      |  height 52, font 16
   +----------------------------------------------------------+
   |  > PostgreSQL                        Database - 320 MB   |  row height 44
   |      dev add postgres                                    |  dim mono 11px
   |  > Postgres GUI Client               Tool - 90 MB        |
   |      dev add pgadmin                                     |
   +----------------------------------------------------------+
   |  up/down navigate | enter open | tab details | esc close  |  footer 28px, muted
   +----------------------------------------------------------+
 Position: horizontally centered, top edge at 18% of screen height. Max 12 rows, then scroll.
 Selected row: surface2 background + 2px accent left bar. No bold change (avoids reflow).

7.5 EMPTY / ERROR / LOADING STATES (use this copy verbatim)
 No results        "Nothing here yet. Press Enter to search the software catalog."
 Offline install   "This needs a connection. Everything already installed still works."
 Low disk          "Not enough space - needs 320 MB, 180 MB free. Free up space?"
 Tier warning      "This app uses about 1 GB of memory. This device has 4 GB.
                    Install anyway | Use a lighter alternative"
 Txn failed        "Installation stopped. Nothing was changed. Try again | Details"
                   (say "nothing was changed" ONLY if rollback actually succeeded;
                    otherwise state exactly what state the system is in)
COPY RULES plain words, no jargon, no error codes in the primary message (codes go in
Details), never blame the user, always offer a next action.

7.6 GUIDED MODE
Default ON for accounts in their first 7 days. Adds cost confirmations, one-line
explanations under actions, and "what does this mean?" links. Toggle in Settings >
Appearance. OFF removes confirmations except destructive ones and hides subtitles.
Same binary, two personalities. This is how one OS serves both a student and a staff engineer.

================================================================================
8. BUILD, PACKAGING, RELEASE
================================================================================

8.1 BUILD PIPELINE (cargo xtask)
   cargo xtask brand     generate brand artifacts
   cargo xtask catalog   validate TOML, check DAG, build catalog.bin + Ed25519 signature
   cargo xtask build     cargo build --release (workspace), strip, split dbgsym
   cargo xtask conf      generate compositor configs, check goldens
   cargo xtask rootfs    debootstrap + packages.list + pinned mise + overlays + binaries
   cargo xtask image     rootfs -> squashfs -> ISO (xorriso, UEFI+BIOS hybrid)
   cargo xtask test      unit + integration + VM boot + budget assertions
   cargo xtask bench     run bench/, compare to budgets, FAIL on regression > 5%
Reproducibility: pin apt versions (snapshot.debian.org), flatpak commits, mise SHA-256,
and Rust toolchain; set SOURCE_DATE_EPOCH. Normalize timestamps in squashfs/ISO. Run
bit-for-bit hash comparison on release-branch builds (G10-audit, not a PR merge blocker).

8.2 VERSIONING
MAJOR.MINOR.PATCH for the product; images tagged forge-1.0.0-YYYYMMDD. Debian security
updates flow in through a WEEKLY CURATED SNAPSHOT: rebuild the image, run the VM matrix,
then publish. Users receive updates via dev update (a transaction on a system bundle).
Never an unattended background upgrade that could break their machine.

8.3 UPDATE UX
 - Checks happen ONLY when the user opens Settings > Software or runs dev update.
 - Show a plain-language changelog, download size, and whether a reboot is needed.
 - Before applying: allowlisted config backup, plus a Btrfs snapshot on Tier 2. On
   failure: auto recovery/rollback.
 - Security updates flagged prominently but still require one click. Optional opt-in
   (default OFF, chosen at OOBE) for library-only patches with no ABI change.

8.4 WHAT SHIPS IN THE ISO
One image, all tiers; compositor chosen at boot (labwc on T0, sway on T1/T2). ISO
<= 2.2 GiB so it fits a 4 GB USB with persistence room. Live session runs toram if
RAM >= 6 GiB, otherwise streams from USB with readahead.

8.5 LICENSING AND COMPLIANCE (day 1, not at launch)
 - Choose our license deliberately (GPLv3 for OS components, or MIT/Apache for tooling if
   adoption matters). File an ADR.
 - Ship NOTICE plus a full source offer for GPL components; host corresponding source
   tarballs for every released image. Automate with cargo xtask source-offer.
 - Trademarks: never use upstream marks in our branding. Upstream names appear only as
   factual attribution in docs/upstream.md and the About page.
 - Keep ID_LIKE=debian (N6). Concealing lineage creates legal and support risk. Branding the
   experience thoroughly is legitimate and sufficient; every successful distro does this.

8.6 v2 PATH (do not build now; do not design against it)
Keep the rootfs recipe declarative so it can later become a Containerfile for bootc-based
atomic delivery [9][13]. Avoid imperative post-install mutations: everything must be
expressible as "files in a tree + units".

================================================================================
9. TEST AND VERIFICATION MATRIX
================================================================================

9.1 REFERENCE HARDWARE (buy these; they are cheap and they define the promise)
 Tier 0      ThinkPad X220/T420: Sandy Bridge, 4 GiB, spinning HDD. THE gate machine.
 Tier 0 alt  any Core 2 Duo era machine, 2-4 GiB. Validates the N5 hard floor.
 Tier 1      ThinkPad T480: 8 GiB, SATA SSD, UHD620.
 Tier 2      any 2023+ machine, 16 GiB, NVMe, modern iGPU/dGPU.
 QEMU profiles emulating each: -cpu SandyBridge, -m 2G, rotational disk via
 rotation_rate=1 plus iops throttling.

9.2 NORMATIVE MEASUREMENT METHODS
 Idle RAM   60 s after desktop ready, no input. Sum PSS from /proc/*/smaps_rollup for all
            user-session processes. Report PSS, not RSS. Script: bench/idle_ram.sh.
 Boot       systemd-analyze (kernel + initrd + userspace to greeter; exclude firmware).
            Video-record the HDD boot as a cross-check.
 Launcher   monotonic timestamp at key event and at frame commit; histogram over 1000
            synthetic keystrokes; report p50/p95/p99.
 Search     criterion bench over a 5000-entry synthetic catalog, cold and warm.
 Frame time compositor frame timing plus WAYLAND_DEBUG sampling; p99 < 2x budget.
 Wakeups    powertop --time=60 at idle, parse wakeups/s.

9.3 TEST PYRAMID
 Unit (every commit)      tier fixtures, search correctness, ranking stability, DAG
                          validation, config goldens, planner purity, journal replay.
 Integration (every commit) daemon+CLI over a real socket against a temp rootfs with stub
                          backends: protocol, errors, rollback, SO_PEERCRED/Polkit checks.
 VM (nightly + release)   boot each QEMU profile, assert 1.3 budgets, run F1-F8 via scripted
                          input, screenshot-diff UI against goldens.
 Hardware (weekly, manual) tests/hardware/*.md checklists on real machines: Wi-Fi,
                          suspend/resume, external monitor, brightness keys, audio, battery.
 Chaos (pre-release)      kill -9 forged mid-transaction; power-cut during apt; fill disk to
                          100%; corrupt catalog.bin; unplug Wi-Fi mid-download. The system
                          must recover or fail cleanly. Never silently corrupt.

9.4 CI GATES (merge blocked if any fails)
 G1  fmt + clippy -D warnings
 G2  all unit and integration tests pass
 G3  catalog schema valid, DAG acyclic, Ed25519 signatures verify
 G4  config goldens match
 G5  budgets within 1.3 on the VM profile; regression > 5% blocks
 G6  no banned runtime (Python/Node/Qt) in shipped Forge artifacts
 G7  systemctl list-units --state=running matches the 2.3 allowlist
 G8  no TCP listeners
 G9  contrast ratios pass
 G10-audit (release-branch / nightly, non-blocking on PRs): reproducible build hash check

================================================================================
10. MILESTONES AND DEFINITION OF DONE
================================================================================
 M0 (W1-4)   Base image boots (trixie + non-free-firmware); idle budget met on T0 VM.
             Stand-in shell (fuzzel/waybar/mako) enabled for early validation. Gate: G7, N1
 M1 (W5-7)   Tiering + unified keymap across labwc and sway.                   Gate: N5, goldens
 M2 (W8-14)  Catalog + transactions + dev CLI + socket-proxy parking.          Gate: F6, chaos
 M3 (W15-22) forge-shell (GTK4 + layer-shell): launcher, pill, notifications.  Gate: latency
 M4 (W21-26) forge-settings incl. Store, Services, Repair.                     Gate: F3, F7
 M5 (W25-30) Installer + OOBE + allowlisted safety net.                        Gate: F1, F2, F8
 M6 (W31-36) Docs, hardware matrix, beta ISO, source offer, G10 audit.         Gate: G1-G9

DEFINITION OF DONE for v1.0
 - All eight functional demos F1-F8 pass on ALL FOUR reference machines.
 - All 1.3 budgets met on the matching tier.
 - Zero known data-loss bugs; every destructive path has a tested recovery path.
 - A stranger installs it and builds a running web app in under 15 minutes, unaided.
 - Source offer published, NOTICE complete, lineage documented.

STAFFING REALITY
 - Team of 3 (systems/Debian, Rust/daemon, GTK4/UX+QA): 9-12 months (36 weeks target).
 - Faster v1.0 option (6-8 months): ship M0-M2 + M4-M6 with themed fuzzel/waybar/mako,
   then ship custom forge-shell in v1.1.
 - Solo engineer: 15-24 months.
Do not compress M2. The catalog, transaction engine, and parking system ARE the product.

================================================================================
APPENDIX A - ANTI-PATTERN CHECKLIST (review before every merge)
================================================================================
 [ ] Did I add a background process?      justify against 2.3 or remove it
 [ ] Did I add a dependency?              measure its PSS and startup cost; do not guess
 [ ] Did I make something CLI-only?       N3 violated
 [ ] Did I remove a capability instead of parking it?   N4 violated
 [ ] Did I hardcode a UI-visible brand string?          0.3 violated
 [ ] Did I add a float or allocation in catalog search? 5.4 violated
 [ ] Did I loop apt per package?          5.5 violated
 [ ] Did I add an animation that blocks input?          7.2 violated
 [ ] Error message with a code but no next action?      7.5 violated
 [ ] Did I skip the acceptance test?      then it is not done

================================================================================
APPENDIX B - GLOSSARY
================================================================================
 Resident   installed and expected to run.
 Declared   in the catalog, not installed, one action away.
 Parked     installed, socket/proxy/path-activated, zero RAM until used.
 Tier       hardware profile 0 / 1 / 2.
 Transaction journaled, recoverable set of install/remove steps (atomic on Tier 2 Btrfs).
 Golden file committed expected output used to detect unintended config changes.
 PSS        proportional set size; the honest memory metric when pages are shared.
 CSR        compressed sparse row; offsets + targets arrays for graph adjacency.

================================================================================
APPENDIX C - SOURCES CONSULTED FOR THIS REVISION
================================================================================
 [2]  Debian Releases - trixie is current stable. https://www.debian.org/releases/
 [3]  Debian release history - trixie released 2025-08-09, kernel 6.12 LTS.
      https://en.wikipedia.org/wiki/Debian_release_version_history
 [4]  Debian trixie release information. https://www.debian.org/releases/trixie/
 [9]  Fedora bootc getting started. https://docs.fedoraproject.org/en-US/bootc/getting-started/
 [13] bootc-dev/bootc - boot and upgrade via container images.
      https://github.com/bootc-dev/bootc
 [18] Debian trixie labwc package (0.8.3-1, wlroots 0.18).
      https://packages.debian.org/stable/x11/labwc
 [20] Debian trixie sway package (1.10.1-2).
      https://packages.debian.org/trixie/sway
 [25] Debian hyprland package (trixie-backports 0.55.2, not trixie main).
      https://packages.debian.org/hyprland
 [26] Debian trixie gtk4-layer-shell (libgtk4-layer-shell0 1.0.4-2).
      https://packages.debian.org/gtk4-layer-shell
 [27] Debian trixie greetd, systemd-boot, thunar packages.
      https://packages.debian.org/trixie/greetd
      https://packages.debian.org/trixie/systemd-boot
      https://packages.debian.org/trixie/thunar
 [33] mise-en-place official site. https://mise.jdx.dev/
 [34] jdx/mise repository. https://github.com/jdx/mise

END OF DOCUMENT FORGE-BP-001 v1.1
