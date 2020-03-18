# gitterm
terminal ui (tui) frontend for git written in rust

![img](assets/main.jpg)

## motivation

I do most of my git usage in a terminal but i frequently found myself using git UIs for some use cases like: index/commit, diff, stash and log
Over the last 2 years my go to GUI tool for this was [fork](https://git-fork.com) because it was not bloated, snappy and free. Unfortunately the *free* part will [change soon](https://github.com/ForkIssues/TrackerWin/issues/571) and so I decided to build a fast & simple terminal tool myself to copy the fork features i am using the most.

# todo

* [x] show files that changed
* [x] show files on index
* [x] colorize diff
* [x] only show diff of selected file
* [x] change detection
* [x] allow scrolling diff
* [x] support staging
* [x] show added files on working dir changes
* [x] support committing
* [ ] popup centered
* [ ] adding a pic to index crashes
* [ ] allow selecting/diff index items
* [ ] support unstaging
* [ ] polling in thread
* [ ] log view

# resources (quick links)

* https://docs.rs/git2/
* https://libgit2.org
* https://docs.rs/tui/
* https://docs.rs/crossterm/

# alternatives

* https://github.com/jesseduffield/lazygit