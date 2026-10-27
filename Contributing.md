# GUIDELINES FOR CONTRIBUTORS

- main.rs should be an entry point and delegate everything outwards to cli and ops, most errors should also propagate back to main.rs unless said error needs to handle its own failure for the sake of safety

- the default rust style is accepted

- try not to use .clone() and unsafe

- follow a strict never nesting rule
  - 5 layers of nesting is forbidden
  - 3 layers of nesting holds a grudge

- protize simplictity over fast complexity, protizie safety over all that

- code needs to be read, so let it breath, dont write hieroglyphs 

- run [typos](https://github.com/crate-ci/typos) to catch spellings mistakes

- follow the DRY (Don't repeat Yourself) technique

- utilize helpers to reduce the density of functions but use them in moderation
 - when its a staged function - yes
 - when its a utility - no

- dont do any programming tricks that require specific knowledge to understand, tricks are fun until you have to patch them at 2AM

- if a struct definition takes too many lines, try to move it to a separate files

- helpers should remain the file they are used and should go into util if 2 files need to use it with context ofcourse,
  - the insert helper stays in the insert command unless the install command needs it, however get_tapes_data can go in utils

- read the previous commit messages to get a sense of how to commit
  - if i dont like the message i'll request a rebase

- you dont have to follow this strictly as its quite leinet but follow the spirit of it

> ## Thank you for contributing to tapes 
