<h1 align="center">
<img width="400px" src="assets/logo.png" />

![CI][s0] [![crates][s1]][l1] ![MIT][s2] [![UNSAFE][s3]][l3]
</h1>

[s0]: https://github.com/extrawurst/gitui/workflows/CI/badge.svg
[s1]: https://img.shields.io/crates/v/gitui.svg
[l1]: https://crates.io/crates/gitui
[s2]: https://img.shields.io/badge/license-MIT-blue.svg
[s3]: https://img.shields.io/badge/unsafe-forbidden-success.svg
[l3]: https://github.com/rust-secure-code/safety-dance/

blazing fast terminal-ui for git written in rust

![](assets/demo.gif)

# features

* fast and intuitive key only control
* context based help (**no** need to remember any hot-key)
* inspect/commit changes (incl. hooks: *commit-msg*/*post-commit*)
* (un)stage files/hunks, revert/reset files/hunk
* scalable ui layout
* async [input polling](assets/perf_compare.jpg) and 
* async git API for fluid control

# known limitations

* hooks don't work on windows (see [#14](https://github.com/extrawurst/gitui/issues/14))
* [core.hooksPath](https://git-scm.com/docs/githooks) config not supported
* revert/reset hunk in working dir (see [#11](https://github.com/extrawurst/gitui/issues/11))

# motivation

I do most of my git usage in a terminal but I frequently found myself using git UIs for some use cases like: index/commit, diff, stash and log.

Over the last 2 years my go-to GUI tool for this was [fork](https://git-fork.com) because it was not bloated, snappy and free. Unfortunately the *free* part will [change soon](https://github.com/ForkIssues/TrackerWin/issues/571) and so I decided to build a fast & simple terminal tool myself to copy the fork features i am using the most.

# installation

For the time being this product is considered alpha and **not** production ready.

## homebrew

```
brew install extrawurst/tap/gitui
```

## install from source

### requirements

install `rust`/`cargo`: https://www.rust-lang.org/tools/install

### cargo install

the simplest way to start playing around with `gitui` is to have `cargo` build/install it:

```
cargo install gitui
```

# diagnostics:

to enable logging to `~/.gitui/gitui.log`:
```
GITUI_LOGGING=true gitui
```

# inspiration

* https://github.com/jesseduffield/lazygit
* https://github.com/jonas/tig
* https://github.com/git-up/GitUp (would be nice to comeup with a way to have the map view available in a terminal tool)
