# Key Config

The default keys are based on arrow keys to navigate.

However popular demand lead to fully customizability of the key bindings.

On first start `gitui` will create `key_config.ron` file automatically based on the defaults.
This file allows changing every key binding.

The config file format based on the [Ron file format](https://github.com/ron-rs/ron).
The location of the file depends on your OS:
* `$HOME/.config/gitui/key_config.ron` (mac)
* `$XDG_CONFIG_HOME/gitui/key_config.ron` (linux using XDG)
* `$HOME/.config/gitui/key_config.ron` (linux)
* `%APPDATA%/gitui/key_config.ron` (Windows)

Here is a [vim style key config](vim_style_key_config.ron) with `h`, `j`, `k`, `l` to navigate. Use it to copy the content into `key_config.ron` to get vim style key bindings.

# Key Symbols

Similar to the above GitUI allows you to change the way the UI visualizes key combos containing special keys like `enter`(default: `⏎`) and `shift`(default: `⇧`).

If we can find a file `key_symbols.ron` in the above folders we apply the overwrites in it.

Example content of this file looks like:

```
(
    enter: Some("enter"),
    shift: Some("shift-")
)
```
This example will only overwrite two symbols. Find all possible symbols to overwrite in `symbols.rs` in the type `KeySymbolsFile` ([src/keys/symbols.rs](https://github.com/extrawurst/gitui/blob/master/src/keys/symbols.rs))