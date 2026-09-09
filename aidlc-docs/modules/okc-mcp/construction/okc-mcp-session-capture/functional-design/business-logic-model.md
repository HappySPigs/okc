# Session capture behavior

Entry: user explicitly selects the session/content; require userSelected=true on each capture call, otherwise return CAPTURE_SELECTION_REQUIRED before scanning/writing. A prior capture is not standing authorization.

Preparation: validate topics -> scan local notes within configured limits -> find session records -> collect organization/folder/section context -> rank lexical candidates -> return evidence and host instructions.

Host: extract only useful session knowledge -> read local candidates and prior records -> select existing sections or new paths -> preserve sources and contradictory evidence -> call apply preview -> inspect preview -> apply under the user's save request -> report receipts. Never ask the user to choose a file merely because the internal tools need a path.

Application: validate inputs -> scan identity locations -> inspect all targets -> build complete per-note proposed content -> run normal authoring validation -> reserve response space -> preview, or sequential atomic writes -> readback -> complete/partial receipt.

State: discovery writes no state. Capture markers survive restart in the source Markdown. Backups remain in the existing external state directory. A failed batch is resumed using fresh preparation and hashes.

