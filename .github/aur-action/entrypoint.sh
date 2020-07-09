#!/bin/sh -l

git clone https://aur.archlinux.org/gitui.git
cd gitui
makepkg --noconfirm -s

echo "done"