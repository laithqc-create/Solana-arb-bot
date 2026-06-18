## Session Management Rules

1. *Monitor token usage* — When you sense the conversation is getting long or context is heavy, proactively warn me.

2. *Before hitting the limit*, always:
   - Save all progress, decisions, and current state to PROGRESS.md
   - Save any unfinished code to their respective files
   - Write a clear "RESUME FROM HERE" section at the bottom of PROGRESS.md

3. *At the start of every session*, always:
   - First action: read PROGRESS.md if it exists
   - Summarize what was done and what's next before doing anything else
   - Ask me to confirm before continuing

4. *PROGRESS.md structure to always follow:*
   - What's been completed
   - Current file being worked on
   - Exact next steps
   - Any blockers or decisions pending

5. LICENSE CHECKING: If you need to reference or use an open-source project to build a feature, verify that its license allows commercial use (e.g., MIT, Apache 2.0, BSD). 

6. CLEAN-ROOM DEVELOPMENT: If a relevant open-source project has a restrictive license (like GPL, AGPL, or CC-BY-NC) and no commercial alternative exists, you must only LEARN from its logic. Do not copy it. Write a completely new, independent implementation from scratch.

7. ARCHITECTURAL INTEGRATION: When adding a new feature, do not copy-paste an existing code block and heavily modify it. Instead, analyze the existing codebase, write clean new code, and properly wire/integrate it into the current system to prevent high CPU usage and code duplication.
8. PRODUCTION READY: Ensure all code includes robust error handling (no silent crashes), strictly validates user inputs, secures all API keys using environment variables, and contains no performance-heavy redundant loops.