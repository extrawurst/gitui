# asyncgit

*allow using git2 in a asynchronous context*

This crate is part of the [gitui](http://gitui.org) project.
It is used put long running [git2](https://github.com/rust-lang/git2-rs) calls onto a thread pool and use [crossbeam-channel](https://github.com/crossbeam-rs/crossbeam) to wait for a message to confirm the call finished.