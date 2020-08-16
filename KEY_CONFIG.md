# Key Config

Default using arrow key to navigate the gitui and Ctrl + C to leave
Here is the [default config](assets/default_key_config.ron)

to change the key bindings of the program you have to modify `key_config.ron` file
[Ron format](https://github.com/ron-rs/ron) located at config path. The path differs depending on the operating system:

* `$HOME/Library/Preferences/gitui/key_config.ron` (mac)
* `$XDG_CONFIG_HOME/gitui/key_config.ron` (linux using XDG)
* `$HOME/.config/gitui/key_config.ron` (linux)

There is also a vim style key config with `h`, `j`, `k`, `l` to navigate and `Ctrl + C` to leave
Here is the [default config](assets/vim_style_key_config.ron)
