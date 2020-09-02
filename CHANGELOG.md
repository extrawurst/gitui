# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased

### Added
- push to origin support ([#265](https://github.com/extrawurst/gitui/issues/265))

## [0.10.1] - 2020-09-01

### Fixed
- static linux binaries broke due to new clipboard feature which is disabled on linux for now ([#259](https://github.com/extrawurst/gitui/issues/259))

## [0.10.0] - 2020-08-29

### Added

- fully **customizable key bindings** (see [KEY_CONFIG.md](KEY_CONFIG.md)) [[@yanganto](https://github.com/yanganto)] ([#109](https://github.com/extrawurst/gitui/issues/109)) ([#57](https://github.com/extrawurst/gitui/issues/57))
- support scrolling in long commit messages [[@cruessler](https://github.com/cruessler)]([#208](https://github.com/extrawurst/gitui/issues/208))

![scrolling](assets/msg-scrolling.gif)

- copy lines from diffs to clipboard [[@cruessler](https://github.com/cruessler)]([#229](https://github.com/extrawurst/gitui/issues/229))

![select-copy](assets/select-copy.gif)

- scrollbar in long diffs ([#204](https://github.com/extrawurst/gitui/issues/204))

![scrollbar](assets/scrollbar.gif)

- allow creating new branch ([#253](https://github.com/extrawurst/gitui/issues/253))

### Fixed

- selection error in stashlist when deleting last element ([#223](https://github.com/extrawurst/gitui/issues/223))
- git hooks broke ci build on windows [[@dr-BEat](https://github.com/dr-BEat)] ([#235](https://github.com/extrawurst/gitui/issues/235))

## [0.9.1] - 2020-07-30

### Added

- move to (un)staged when the current selection is empty [[@jonstodle](https://github.com/jonstodle)]([#215](https://github.com/extrawurst/gitui/issues/215))
- pending load of a diff/status is visualized ([#160](https://github.com/extrawurst/gitui/issues/160))
- entry on [git-scm.com](https://git-scm.com/downloads/guis) in the list of GUI tools [[@Vidar314](https://github.com/Vidar314)] (see [PR](https://github.com/git/git-scm.com/pull/1485))
- commits can be tagged in revlog [[@cruessler](https://github.com/cruessler)]([#103](https://github.com/extrawurst/gitui/issues/103))

![](assets/tagging.gif)

### Changed

- async fetching tags to improve reactivity in giant repos ([#170](https://github.com/extrawurst/gitui/issues/170))

### Fixed

- removed unmaintained dependency `spin` ([#172](https://github.com/extrawurst/gitui/issues/172))
- opening relative paths in external editor may fail in subpaths ([#184](https://github.com/extrawurst/gitui/issues/184))
- crashes in revlog with utf8 commit messages ([#188](https://github.com/extrawurst/gitui/issues/188))
- `add_to_ignore` failed on files without a newline at EOF ([#191](https://github.com/extrawurst/gitui/issues/191))
- new tags were not picked up in revlog view ([#190](https://github.com/extrawurst/gitui/issues/190))
- tags not shown in commit details popup ([#193](https://github.com/extrawurst/gitui/issues/193))
- min size for relative popups on small terminals ([#179](https://github.com/extrawurst/gitui/issues/179))
- fix crash on resizing terminal to very small width ([#198](https://github.com/extrawurst/gitui/issues/198))
- fix broken tags when using a different internal representation ([#206](https://github.com/extrawurst/gitui/issues/206))
- tags are not cleanly seperated in details view ([#212](https://github.com/extrawurst/gitui/issues/212))

## [0.8.1] - 2020-07-07

### Added

- open file in editor [[@jonstodle](https://github.com/jonstodle)]([#166](https://github.com/extrawurst/gitui/issues/166))

### Fixed

- switch deprecated transitive dependency `net2`->`socket2` [in `crossterm`->`mio`]([#66](https://github.com/extrawurst/gitui/issues/66))
- crash diffing a stash that was created via cli ([#178](https://github.com/extrawurst/gitui/issues/178))
- zero delta file size in diff of untracked binary file ([#171](https://github.com/extrawurst/gitui/issues/171))
- newlines not visualized correctly in commit editor ([#169](https://github.com/extrawurst/gitui/issues/169))

![](assets/newlines.gif)

## [0.8.0] - 2020-07-06

### Added

- core homebrew [formulae](https://formulae.brew.sh/formula/gitui#default): `brew install gitui` [[@vladimyr](https://github.com/vladimyr)](<[#137](https://github.com/extrawurst/gitui/issues/137)>)
- show file sizes and delta on binary diffs ([#141](https://github.com/extrawurst/gitui/issues/141))

![](assets/binary_diff.png)

- external editor support for commit messages [[@jonstodle](https://github.com/jonstodle)](<[#46](https://github.com/extrawurst/gitui/issues/46)>)

![](assets/vi_support.gif)

### Changed

- use terminal blue as default selection background ([#129](https://github.com/extrawurst/gitui/issues/129))
- author column in revlog is now fixed width for better alignment ([#148](https://github.com/extrawurst/gitui/issues/148))
- cleaner tab bar and background work indicating spinner:

![](assets/spinner.gif)

### Fixed

- clearer help headers ([#131](https://github.com/extrawurst/gitui/issues/131))
- display non-utf8 commit messages at least partially ([#150](https://github.com/extrawurst/gitui/issues/150))
- hooks ignored when running `gitui` in subfolder of workdir ([#151](https://github.com/extrawurst/gitui/issues/151))
- better scrolling in file-trees [[@tisorlawan](https://github.com/tisorlawan)]([#144](https://github.com/extrawurst/gitui/issues/144))
- show untracked files in stash commit details [[@MCord](https://github.com/MCord)]([#130](https://github.com/extrawurst/gitui/issues/130))
- in some repos looking up the branch name was a bottleneck ([#159](https://github.com/extrawurst/gitui/issues/159))
- some optimizations in reflog
- fix arrow utf8 encoding in help window [[@daober](https://github.com/daober)]([#142](https://github.com/extrawurst/gitui/issues/142))

## [0.7.0] - 2020-06-15

### Added

- Inspect stash commit in detail ([#121](https://github.com/extrawurst/gitui/issues/121))
- Support reset/revert individual hunks ([#11](https://github.com/extrawurst/gitui/issues/11))
- Commit Amend (`ctrl+a`) when in commit popup ([#89](https://github.com/extrawurst/gitui/issues/89))

![](assets/amend.gif)

### Changed

- file trees: `arrow-right` on expanded folder moves down into folder
- better scrolling in diff ([#52](https://github.com/extrawurst/gitui/issues/52))
- display current branch in status/log ([#115](https://github.com/extrawurst/gitui/issues/115))
- commit msg popup: add cursor and more controls (`arrow-left/right`, `delete` & `backspace`) [[@alistaircarscadden](https://github.com/alistaircarscadden)]([#46](https://github.com/extrawurst/gitui/issues/46))
- moved `theme.ron` from `XDG_CACHE_HOME` to `XDG_CONFIG_HOME` [[@jonstodle](https://github.com/jonstodle)](<[#98](https://github.com/extrawurst/gitui/issues/98)>)

### Fixed

- reset file inside folder failed when running `gitui` in a subfolder too ([#118](https://github.com/extrawurst/gitui/issues/118))
- selection could disappear into collapsed folder ([#120](https://github.com/extrawurst/gitui/issues/120))
- `Files: loading` sometimes wrong ([#119](https://github.com/extrawurst/gitui/issues/119))

## [0.6.0] - 2020-06-09

![](assets/commit-details.gif)

### Changed

- changed hotkeys for selecting stage/workdir (**Note:** use `[w]`/`[s]` to change between workdir and stage) and added hotkeys (`[1234]`) to switch to tabs directly ([#92](https://github.com/extrawurst/gitui/issues/92))
- `arrow-up`/`down` on bottom/top of status file list switches focus ([#105](https://github.com/extrawurst/gitui/issues/105))
- highlight tags in revlog better

### Added

- New `Stage all [a]`/`Unstage all [a]` in changes lists ([#82](https://github.com/extrawurst/gitui/issues/82))
- add `-d`, `--directory` options to set working directory via program arg [[@alistaircarscadden](https://github.com/alistaircarscadden)]([#73](https://github.com/extrawurst/gitui/issues/73))
- commit detail view in revlog ([#80](https://github.com/extrawurst/gitui/issues/80))

### Fixed

- app closes when staging invalid file/path ([#108](https://github.com/extrawurst/gitui/issues/108))
- `shift+tab` not working on windows [[@MCord](https://github.com/MCord)]([#111](https://github.com/extrawurst/gitui/issues/111))

## [0.5.0] - 2020-06-01

### Changed

- support more commands allowing optional multiline commandbar ([#83](https://github.com/extrawurst/gitui/issues/83))

![](assets/cmdbar.gif)

### Added

- support adding untracked file/folder to `.gitignore` ([#44](https://github.com/extrawurst/gitui/issues/44))
- support reverse tabbing using shift+tab ([#92](https://github.com/extrawurst/gitui/issues/92))
- switch to using cmd line args instead of `ENV` (`-l` for logging and `--version`) **please convert your GITUI_LOGGING usage** [[@shenek](https://github.com/shenek)]([#88](https://github.com/extrawurst/gitui/issues/88))
- added missing LICENSE.md files in sub-crates [[@ignatenkobrain](https://github.com/ignatenkobrain)]([#94](https://github.com/extrawurst/gitui/pull/94))

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
- more error/panic resiliance (`unwrap`/`panic` denied by clippy now) [[@MCord](https://github.com/MCord)](<[#77](https://github.com/extrawurst/gitui/issues/77)>)

### Fixes

- panic on small terminal width ([#72](https://github.com/extrawurst/gitui/issues/72))

![](assets/stashing.gif)

## [0.3.0] - 2020-05-20

### Added

- support color themes and light mode [[@MCord](https://github.com/MCord)]([#28](https://github.com/extrawurst/gitui/issues/28))

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
- hook support on windows [[@MCord](https://github.com/MCord)]([#14](https://github.com/extrawurst/gitui/issues/14))

### Changed

- show longer commit messages in log view
- introduce propper error handling in `asyncgit` [[@MCord](https://github.com/MCord)]([#53](https://github.com/extrawurst/gitui/issues/53))
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
