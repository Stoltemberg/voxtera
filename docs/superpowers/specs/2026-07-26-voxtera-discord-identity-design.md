# Voxtera Discord identity update

## Goal

Make Discord Rich Presence identify the running game as Voxtera rather than
Veloren.

## Design

- Replace only the Discord Application ID in `voxygen/src/discord.rs` with
  the user-provided Voxtera Application ID.
- Do not store, transmit or reference the Discord public key; Rich Presence
  uses the Application ID only.
- Keep the existing integration behavior, activity text, setting toggle and
  update interval unchanged.
- Do not make unrelated branding or localisation changes in this task.

## Acceptance criteria

- The Rust Discord client is initialized with the Voxtera Application ID.
- `DISCORD_APP_ID` remains a valid `discord_sdk::AppId` constant.
- No public key or credential is added to the repository.
