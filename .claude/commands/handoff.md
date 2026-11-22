---
description: Save progress on the current issue so a fresh session can resume it
---

Save progress on the current issue so the user can `/clear` and resume with `/implement <n>`.

1. Find the issue number from the branch name `issue-<n>-*`.
2. Commit all work in progress on that branch (message starts with `WIP:`), even if tests fail, and push the branch.
3. Post a comment on issue #n (`gh issue comment <n> --body-file <file>`) in this shape:

   ```
   Handoff <date>

   Done:
   - ...
   Next step:
   - the single exact thing to do first
   Remaining:
   - ...
   Gotchas:
   - anything learned that is not in the code or the issue
   Tests: <passing / failing, and which>
   ```

4. If this is the issue's second handoff, it is too big: suggest splitting "Remaining" into a new issue, and create it only if the user agrees.
5. Tell the user: "Saved. Run /clear, then /implement <n>."
