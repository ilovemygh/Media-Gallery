# Agent Instructions (`AGENTS.md`)

All AI agents working in the **Media-Gallery** workspace MUST read and strictly follow these instructions before starting any task, during execution, and upon completion.

---

## 1. Read the Product Requirements Document (PRD) First

- **Mandatory Reading:** Before planning, designing, or writing any code, always read [`PRD-Media-Gallery.md`](./PRD-Media-Gallery.md).
- **Source of Truth:** Treat `PRD-Media-Gallery.md` as the single source of truth for product scope, target platforms (Desktop + Android), architecture, UI/UX behavior, and v1 goals vs. non-goals.
- **Alignment Check:** Verify that every feature, refactor, or file change aligns with the requirements defined in `PRD-Media-Gallery.md`. If a user request conflicts with or extends the PRD, note it clearly and keep the PRD updated if instructed.

---

## 2. Strict Git & GitHub Workflow (Highest Priority)

Pay close attention to Git and GitHub at all times. Never leave uncommitted changes or unpushed tasks behind.

### A. Check Git Status Before Starting
Before touching any files on a task, inspect the repository state:
```bash
git status
git branch
git log -n 5 --oneline
git remote -v
```
- Ensure your local branch is clean and synchronized with the remote (`origin/main` or the active feature branch).
- Check for any untracked, modified, or staged files before making edits.

### B. Commit Every File Change, Addition, or Removal Immediately
Whenever you **create (add)**, **modify (change)**, or **delete (remove)** a file:
1. **Stage the specific file(s):**
   ```bash
   git add <file-path>
   ```
   *(If removing a file, ensure the deletion is staged via `git rm <file-path>` or `git add -u`.)*
2. **Commit immediately with a clear explanation:**
   Every commit message **must clearly explain what the changes did and why**. Avoid vague messages like `"update"`, `"fix"`, or `"wip"`.
   ```bash
   git commit -m "<type>(<scope>): <concise summary of change>" -m "<detailed explanation of what was added, modified, or removed and what the changes accomplish>"
   ```
   - Keep commits granular—do not bundle unrelated file changes into one massive commit at the end.
   - Verify each commit with `git status` and `git log -n 1` to ensure the change was recorded properly.

### C. Push to GitHub After Every Task Is Finished
A task is **not complete** until all commits are pushed to GitHub:
1. Run `git status` to confirm there are **zero** untracked or uncommitted changes remaining (`working tree clean`).
2. Push to the GitHub remote (`https://github.com/ilovemygh/Media-Gallery.git`):
   ```bash
   git push origin main
   ```
   *(Or push to the active working branch if working on a feature branch.)*
3. Confirm the push succeeded and verify synchronization:
   ```bash
   git status
   ```
   Ensure the output confirms your branch is up to date with `origin/<branch>`.

---

## 3. Pre-Completion Checklist for Every Task

Before reporting back to the user that a task is done, verify every item below:
- [ ] Did I read and follow [`PRD-Media-Gallery.md`](./PRD-Media-Gallery.md)?
- [ ] Was every added, modified, or removed file staged and committed with a clear explanation of what the change did?
- [ ] Is `git status` completely clean (`nothing to commit, working tree clean`)?
- [ ] Have all commits for this task been pushed to GitHub (`git push`)?
