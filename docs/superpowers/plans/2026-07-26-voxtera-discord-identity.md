# Voxtera Discord Identity Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Configure Discord Rich Presence to use the Voxtera Discord application.

**Architecture:** The integration already creates `discord_sdk::DiscordApp::PlainId(DISCORD_APP_ID)`. Change only that typed constant. No public key, environment variable, network setting or activity copy is required.

**Tech Stack:** Rust, `discord-sdk`, Cargo.

## Global Constraints

- Replace only the Discord Application ID in `voxygen/src/discord.rs`.
- Use the user-provided Voxtera Application ID `1531103303063175259`.
- Do not store, log, transmit or reference the Discord public key.
- Preserve existing Discord activity behavior and all other game branding.

---

### Task 1: Point Rich Presence at the Voxtera application

**Files:**
- Modify: `voxygen/src/discord.rs:15-19`

**Interfaces:**
- Consumes: `discord_sdk::DiscordApp::PlainId(DISCORD_APP_ID)`.
- Produces: a valid `ds::AppId` constant for the Voxtera Discord application.

- [ ] **Step 1: Write a failing static contract check**

Run:

```powershell
rg -n -F 'const DISCORD_APP_ID: ds::AppId = 1531103303063175259;' voxygen/src/discord.rs
```

Expected: exit code 1 because the file still contains the Veloren application ID.

- [ ] **Step 2: Replace only the application ID**

Change the constant to:

```rust
const DISCORD_APP_ID: ds::AppId = 1531103303063175259;
```

Do not change the surrounding `Discord::start` handshake, activity updates, i18n, public key handling or user settings.

- [ ] **Step 3: Verify the contract and compile the affected crate**

Run:

```powershell
rg -n -F 'const DISCORD_APP_ID: ds::AppId = 1531103303063175259;' voxygen/src/discord.rs
rg -n -F 'a3caad9c38efcea0d7b258acd5451a911f2b224c1440b194cfb6f50b8b5f442d' voxygen
cargo check -p veloren-voxygen
```

Expected: the new constant is found once, the public key search has no matches, and Cargo succeeds.

- [ ] **Step 4: Commit the isolated change**

```powershell
git add voxygen/src/discord.rs
git commit -m "fix: use Voxtera Discord application"
```

## Plan self-review

- **Spec coverage:** Task 1 changes exactly the typed identity constant, verifies it and proves the supplied public key is not committed.
- **Placeholder scan:** No deferred work or unspecified value remains.
- **Interface consistency:** `DISCORD_APP_ID` stays a `ds::AppId` consumed by the existing `PlainId` constructor.

