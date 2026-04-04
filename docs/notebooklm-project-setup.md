# NotebookLM Project Setup

This project already has the NotebookLM MCP server wired in through `opencode.json`.
What remains is the one-time Google authentication step and the project notebook itself.

## 1. Authenticate NotebookLM MCP

In chat, say:

```text
Log me in to NotebookLM
```

That opens Chrome for Google sign-in.

## 2. Create The SuperRoutePro Notebook

Open `https://notebooklm.google.com/` and create a notebook for this repo.

Recommended notebook metadata:

- Name: `SuperRoutePro Project Knowledge`
- Description: `Grounded project knowledge for SuperRoutePro covering architecture, release history, optimization progress, repair flows, route persistence, and current implementation decisions.`
- Topics:
  - `route persistence`
  - `repair mode`
  - `network routing`
  - `speed test`
  - `optimization roadmap`
- Use cases:
  - `verify architecture before refactors`
  - `check recent release decisions`
  - `confirm repair and persistence behavior`
  - `research current implementation details before coding`

## 3. Upload Recommended Source Files

Start with these repo files:

- `Daily_Log.md`
- `OPTIMIZE.md`
- `README.md`
- `CHANGELOG.md`
- `AGENTS.md`
- `PROJECT_SUMMARY.md`
- `NeedToDo.md`
- `IMPLEMENTATION_SUMMARY_VI.md`
- `SETUP_GUIDE_VI.md`

`Daily_Log.md` is the highest-priority source because it tracks the latest validated implementation work.

## 4. Share The Notebook

In NotebookLM:

1. Open the notebook
2. Click `Share`
3. Choose `Anyone with the link`
4. Copy the share URL

## 5. Add It To The Local Library

Once you have the NotebookLM share URL, tell the agent to add it to the local library.

Suggested prompt:

```text
Add this NotebookLM to the SuperRoutePro project library: <share-url>
```

## 6. Working Pattern For This Repo

- Use `Daily_Log.md` as the ongoing source-of-truth after each meaningful session
- Refresh the notebook sources when major releases, architecture changes, or optimization milestones land
- Ask with `use notebooklm` whenever the task depends on current project knowledge
