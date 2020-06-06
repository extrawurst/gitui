# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed
- changed hotkeys for selecting stage/workdir (**Note:** use `[w]`/`[s]` to change between workdir and stage) and added hotkeys (`[1234]`) to switch to tabs directly ([#92](https://github.com/extrawurst/gitui/issues/92))
- `arrow-up`/`down` on bottom/top of status file list switches focus ([#105](https://github.com/extrawurst/gitui/issues/105))
- highlight tags in revlog better

### Added
- New `Stage all [a]`/`Unstage all [a]` in changes lists ([#82](https://github.com/extrawurst/gitui/issues/82))
- add `-d`, `--directory` options to set working directory ([#73](https://github.com/extrawurst/gitui/issues/73))
- commit detail view in revlog ([#80](https://github.com/extrawurst/gitui/issues/80))

### Fixed
- app closes when staging invalid file/path ([#108](https://github.com/extrawurst/gitui/issues/108))
- `shift+tab` not working on windows [[@MCord](https://github.com/MCord)] ([#111](https://github.com/extrawurst/gitui/issues/111))

## [0.5.0] - 2020-06-01

### Changed
- support more commands allowing optional multiline commandbar ([#83](https://github.com/extrawurst/gitui/issues/83))

![](assets/cmdbar.gif)

### Added
- support adding untracked file/folder to `.gitignore` ([#44](https://github.com/extrawurst/gitui/issues/44))
- support reverse tabbing using shift+tab ([#92](https://github.com/extrawurst/gitui/issues/92))
- switch to using cmd line args instead of `ENV` (`-l` for logging and `--version`) **please convert your GITUI_LOGGING usage** [[@shenek](https://github.com/shenek)] ([#88](https://github.com/extrawurst/gitui/issues/88))
- added missing LICENSE.md files in sub-crates [[@ignatenkobrain](https://github.com/ignatenkobrain)] ([#94](https://github.com/extrawurst/gitui/pull/94))

### Fixed
- error when diffing huge files ([#96](https://github.com/extrawurst/gitui/issues/96))
- expressive error when run in bare repos ([#100](https://github.com/extrawurst/gitui/issues/100))

## [0.4.0] - 2020-05-25

### Added
- stashing support (save,apply,drop) ([#3](https://github.com/extrawurst/gitui/issues/3))

### Changed
- log tab refreshes when head changes ([#78](https://github.com/extrawurst/gitui/issues/78))
- performance optimization of the log tab in big repos
- more readable default color for the commit hash in the log tab
- more error/panic resiliance (`unwrap`/`panic` denied by clippy now) [[@MCord](https://github.com/MCord)] ([#77](https://github.com/extrawurst/gitui/issues/77))

### Fixes
- panic on small terminal width ([#72](https://github.com/extrawurst/gitui/issues/72))

![](assets/stashing.gif)

## [0.3.0] - 2020-05-20

### Added
- support color themes and light mode [[@MCord](https://github.com/MCord)] ([#28](https://github.com/extrawurst/gitui/issues/28))

### Changed
- more natural scrolling in log tab ([#52](https://github.com/extrawurst/gitui/issues/52))

### Fixed
- crash on commit when git name was not set ([#74](https://github.com/extrawurst/gitui/issues/74))
- log tab shown empty in single commit repos ([#75](https://github.com/extrawurst/gitui/issues/75))

![](assets/light-theme.png)

## [0.2.6] - 2020-05-18
### Fixed
- fix crash help in small window size ([#63](https://github.com/extrawurst/gitui/issues/63))

## [0.2.5] - 2020-05-16
### Added
- introduced proper changelog
- hook support on windows [[@MCord](https://github.com/MCord)] ([#14](https://github.com/extrawurst/gitui/issues/14))

### Changed
- show longer commit messages in log view
- introduce propper error handling in `asyncgit` [[@MCord](https://github.com/MCord)] ([#53](https://github.com/extrawurst/gitui/issues/53))
- better error message when trying to run outside of a valid git repo ([#56](https://github.com/extrawurst/gitui/issues/56))
- improve ctrl+c handling so it is checked first and no component needs to worry of blocking it

### Fixed
- support multiple tags per commit in log ([#61](https://github.com/extrawurst/gitui/issues/61))

## [0.2.3] - 2020-05-12
### Added
- support more navigation keys: home/end/pageUp/pageDown ([#43](https://github.com/extrawurst/gitui/issues/43))
- highlight current tab a bit better

## [0.2.2] - 2020-05-10
### Added
- show tags in commit log ([#47](https://github.com/extrawurst/gitui/issues/47))
- support home/end key in diff ([#43](https://github.com/extrawurst/gitui/issues/43))

### Changed
- close application shortcut is now the standard `ctrl+c`
- some diff improvements ([#42](https://github.com/extrawurst/gitui/issues/42))

### Fixed
- document tab key to switch tabs ([#48](https://github.com/extrawurst/gitui/issues/48))
