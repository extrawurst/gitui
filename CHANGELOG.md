# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]
### Added
- introduced proper changelog
- hook support on windows [[@MCord](https://github.com/MCord)] ([#14](https://github.com/extrawurst/gitui/issues/14))

### Changed
- show longer commit messages in log view
- introduce propper error handling in `asyncgit` [[@MCord](https://github.com/MCord)] ([#53](https://github.com/extrawurst/gitui/issues/53))
- better error message when trying to run outside of a valid git repo ([#56](https://github.com/extrawurst/gitui/issues/56))
- improve ctrl+c handling so it is checked first and no component needs to worry of blocking it

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
