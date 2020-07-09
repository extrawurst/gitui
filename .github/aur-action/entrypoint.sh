#!/bin/sh -l

pwd
whoami

cd /home/aur
git clone https://aur.archlinux.org/gitui.git
cd gitui
makepkg --noconfirm -s

echo "done"