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
