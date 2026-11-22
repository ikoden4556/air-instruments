---
description: Start or resume work on a GitHub issue
argument-hint: <issue number>
---

Work on GitHub issue #$ARGUMENTS of this repo.

1. Read the issue and all its comments: `gh issue view $ARGUMENTS --comments`. The issue body is the spec; the newest comment starting with "Handoff" is where the last session stopped.
2. Find the branch `issue-$ARGUMENTS-*` (`git branch -a`).
   - If it exists: check it out, read `git log main..HEAD --oneline` and `git diff main...HEAD --stat`, run the tests, and continue from the handoff's "Next step". Don't redo finished work.
   - If not: branch from an up-to-date main as `issue-$ARGUMENTS-<short-slug>`.
3. Read only the parts of `BLUEPRINT.md`, `GLOSSARY.md` and `docs/adr/` the issue links to.
4. Build it. Core (`core/`) work is test-first with the `tdd` skill. Use glossary terms in names. Commit in small steps.
5. When every "Done when" item is met: run the full tests, push the branch, and open a PR with the `pr` skill whose body includes `Closes #$ARGUMENTS`. Ask the user before pushing or opening the PR.
6. If the context window is getting full before you finish, tell the user to run `/handoff`.
