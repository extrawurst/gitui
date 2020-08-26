# Key Config

Default using arrow key to navigate the gitui and Ctrl + C to quit the program

The first time Gitui will create `key_config.ron` file automatically.
You can change the every single key bindings of the program as what you like.

The config file format is [Ron format](https://github.com/ron-rs/ron). 
And the path differs depending on the operating system:
* `$HOME/Library/Preferences/gitui/key_config.ron` (mac)
* `$XDG_CONFIG_HOME/gitui/key_config.ron` (linux using XDG)
* `$HOME/.config/gitui/key_config.ron` (linux)

Here is a [vim style key config](assets/vim_style_key_config.ron) with `h`, `j`, `k`, `l` to navigate and `Ctrl + C` to leave.
You can use it to replace `key_config.ron` and get a vim style setting.
