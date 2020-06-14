# Themes 

default on light terminal:
![](assets/light-theme.png)

to change the colors of the program you have to modify `theme.ron` file
[Ron format](https://github.com/ron-rs/ron) located at config path. The path differs depending on the operating system:

* `$HOME/Library/Preferences/gitui/theme.ron` (mac)
* `$XDG_CONFIG_HOME/gitui/theme.ron` (linux using XDG)
* `$HOME/.config/gitui/theme.ron` (linux)

Valid colors can be found in [ColorDef](./src/ui/style.rs#ColorDef) struct. note that rgb colors might not be supported 
in every terminal.
