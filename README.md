# gitui

![CI](https://github.com/extrawurst/gitui/workflows/CI/badge.svg) [![MIT][s2]][l2] 

[s2]: https://img.shields.io/badge/license-MIT-blue.svg
[l2]: LICENSE

blazing fast terminal-ui for git written in rust

![img](assets/main.jpg)
![img](assets/demo.gif)

## motivation

I do most of my git usage in a terminal but I frequently found myself using git UIs for some use cases like: index/commit, diff, stash and log.

Over the last 2 years my go-to GUI tool for this was [fork](https://git-fork.com) because it was not bloated, snappy and free. Unfortunately the *free* part will [change soon](https://github.com/ForkIssues/TrackerWin/issues/571) and so I decided to build a fast & simple terminal tool myself to copy the fork features i am using the most.

## installation

For the time being this product is considered alpha and not production ready, therefore I do not distribute binary versions yet, however feel free to build `gitui` and let me know what you think!

### requirements

install `rust`/`cargo`: https://www.rust-lang.org/tools/install

### build from source

the simplest way to start playing around with `gitui` is to have `cargo` install it locally:

```
cargo install --path "."
```

after that you can go to your git repo and run it:

```
gitui
```

# todo

* [x] (un)stage files
* [x] inspect diffs
* [x] commit
* [x] [input polling in thread](assets/perf_compare.jpg)
* [x] async git API for fluid control
* [ ] discard untracked files (remove)
* [ ] (un)staging selected hunks

# alternatives

* https://github.com/jesseduffield/lazygit