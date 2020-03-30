# gitui

![CI][s0] [![crates][s1]][l1] ![MIT][s2] [![LOC][s3]][l3]

[s0]: https://github.com/extrawurst/gitui/workflows/CI/badge.svg
[s1]: https://img.shields.io/crates/v/gitui.svg
[l1]: https://crates.io/crates/gitui
[s2]: https://img.shields.io/badge/license-MIT-blue.svg
[s3]: https://tokei.rs/b1/github/extrawurst/gitui
[l3]: https://github.com/extrawurst/gitui

blazing fast terminal-ui for git written in rust

![img](assets/demo.gif)

## features

* fast and intuitive key only control
* context based help (**no** need to remember any hot-key)
* inspect/commit changes
* (un)stage files, revert/reset files
* scalable ui layout
* async [input polling](assets/perf_compare.jpg) and 
* async git API for fluid control

## motivation

I do most of my git usage in a terminal but I frequently found myself using git UIs for some use cases like: index/commit, diff, stash and log.

Over the last 2 years my go-to GUI tool for this was [fork](https://git-fork.com) because it was not bloated, snappy and free. Unfortunately the *free* part will [change soon](https://github.com/ForkIssues/TrackerWin/issues/571) and so I decided to build a fast & simple terminal tool myself to copy the fork features i am using the most.

## installation

For the time being this product is considered alpha and not production ready, therefore I do not distribute binary versions yet, however feel free to build `gitui` and let me know what you think!

### requirements

install `rust`/`cargo`: https://www.rust-lang.org/tools/install

### cargo install

the simplest way to start playing around with `gitui` is to have `cargo` build/install it:

```
cargo install gitui
```

### diagnostics:

to enable logging to `~/.gitui/gitui.log`:
```
GITUI_LOGGING=true gitui
```

# todo for 0.1 (first release)

* [ ] panic on exit (thread sending error)
* [ ] better help command 
* [ ] -> fix: dont show scroll option when any popup open
* [ ] confirm destructive commands (revert/reset)
* [ ] (un)staging selected hunks
* [ ] publish as homebrew-tap

# inspiration

* https://github.com/jesseduffield/lazygit
* https://github.com/jonas/tig
* https://github.com/git-up/GitUp (would be nice to comeup with a way to have the map view available in a terminal tool)
