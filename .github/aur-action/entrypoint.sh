#!/bin/sh -l

pwd

su aur
cd /home/aur
git clone https://aur.archlinux.org/gitui.git
cd gitui
makepkg --noconfirm -s

echo "done"