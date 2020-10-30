# Themes

default on light terminal:
![](assets/light-theme.png)

to change the colors of the program you have to modify `theme.ron` file
[Ron format](https://github.com/ron-rs/ron) located at config path. The path differs depending on the operating system:

* `$HOME/Library/Application Support/gitui/theme.ron` (mac)
* `$XDG_CONFIG_HOME/gitui/theme.ron` (linux using XDG)
* `$HOME/.config/gitui/theme.ron` (linux)

Valid colors can be found in tui-rs' [Color](https://docs.rs/tui/0.12.0/tui/style/enum.Color.html) struct. note that rgb colors might not be supported in every terminal.
