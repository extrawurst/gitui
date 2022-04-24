

## <a name="table-of-contents"></a> Table of Contents

1. ["Bad Credentials" Error](#credentials)
2. [Custom key bindings](#keybindings)

## 1. <a name="credentials"></a> "Bad Credentials" Error <small><sup>[Top ▲](#table-of-contents)</sup></small>

Some users have trouble pushing/pulling from remotes and adding their ssh-key to their ssh-agent solved the issue. The error they get is:
![](./assets/bad-credentials.png)

See gihtub's excellent documentation for [Adding your SSH Key to the ssh-agent](https://docs.github.com/en/authentication/connecting-to-github-with-ssh/generating-a-new-ssh-key-and-adding-it-to-the-ssh-agent#adding-your-ssh-key-to-the-ssh-agent)

Note that in some cases adding the line `ssh-add -K ~/.ssh/id_ed25519`(or whatever your key is called) to your bash init script is necessary too to survive restarts.

## 2. <a name="keybindings"></a> Custom key bindings <small><sup>[Top ▲](#table-of-contents)</sup></small>

If you want to use `vi`-style keys or customize your key bindings in any other fassion see the specific docs on that: [key config](./KEY_CONFIG.md)

