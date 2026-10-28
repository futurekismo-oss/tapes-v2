# GUIDELINES FOR CONTRIBUTORS

- `main.rs` should be an entry point and delegate everything outwards to `cli` and `ops`. Most errors should also propagate back to `main.rs`, unless a specific error needs to handle its own failure for the sake of safety.

- The default Rust style is accepted.

- Try not to use `.clone()` and `unsafe`.

- Follow a strict never-nesting rule:
  - 5 layers of nesting is forbidden.
  - 3 layers of nesting holds a grudge.

- Prioritize simplicity over fast complexity, and prioritize safety above all else.

- Code needs to be read, so let it breathe; don't write hieroglyphs.

- Run [typos](https://github.com/crate-ci/typos) to catch spelling mistakes.

- Follow the DRY (Don't Repeat Yourself) technique.

- Utilize helpers to reduce the density of functions, but use them in moderation:
  - When it's a staged function — yes.
  - When it's a utility — no.

- Don't use any programming tricks that require specific knowledge to understand. Tricks are fun until you have to patch them at 2 AM.

- If a struct definition takes too many lines, try to move it to a separate file.

- Try to keep helpers in the file where they are used. They should only go into `util` if two files need to use them (with context, of course).
  - The `insert` helper stays in the `insert` command unless the `install` command needs it; however, `get_tapes_data` can go in `utils`.

- Try not to write code blind, test, test, test.
  - This repo is currently lacking in test so that might be the first place to commit

- Read the previous commit messages to get a sense of how to commit.
  - If I don't like the message, I'll request a rebase.

- You don't have to follow this strictly, as it's quite lenient, but please follow the spirit of it.

- AI is skeptically allowed in this repo:
  - You must understand the code fully.
  - Be liable for any errors.
  - Fully agentic commits are not allowed.
  - Use AI the same way you would use Stack Overflow if your `Ctrl+C` and `Ctrl+V` keys didn't work.

> ## Thank you for contributing to tapes!
