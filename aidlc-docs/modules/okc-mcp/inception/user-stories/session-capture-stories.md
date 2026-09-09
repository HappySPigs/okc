# Session capture stories

User-facing workflow warrants stories. Persona: a developer working with a coding agent; the agent is the tool caller. Existing vault owner and knowledge-reader personas remain unchanged.

- [x] SC-1: Saying “record this session” causes the agent to discover existing local knowledge and choose destinations without asking for file paths. REQ-023/024.
- [x] SC-2: A finding belonging to an existing topic is added under the relevant section, preserving surrounding content and source links. REQ-026/027.
- [x] SC-3: A session with several independent topics can update several notes or create a new topic note. REQ-026.
- [x] SC-4: Capturing the same session again updates its prior blocks without duplicate records, including after reconnect. REQ-025.
- [x] SC-5: Preview, conflict detection, backups and partial-result reporting make capture changes recoverable. REQ-027/028.
- [x] SC-6: A configured published web Vault remains available for knowledge reads while placement uses the writable local original. REQ-024/028.

Story planning/generation follow the user's explicit implementation direction and existing module autopilot. The examples pin observable persistence behavior; semantic selection quality remains a host-agent evaluation boundary.


- [x] SC-7: Only the session/content explicitly selected by the user is captured. Missing/false userSelected rejects before scanning/writing, and a completed capture never authorizes a later unselected one. REQ-030; domain and stdio negative tests passed.
