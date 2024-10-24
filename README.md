# Universal Entity Tracking System

## 🏗️ Building and Running

1. Set up a toolbox container.
   - Run, `toolbox create --image quay.io/toolbx-images/debian-toolbox:12`
2. Set up Rust via `rustup`.
   - Optionally, install `rust-analyzer` via `rustup component add rust-analyzer`.
3. Install the required dependencies.

```sh
sudo apt install libgtk-4-dev libadwaita-1-dev
```

4. Use `./run` to build and run the project.
   - `./run`

## 🔃 Syncing code to the Pi

```sh
rsync --filter=':- .gitignore' --exclude \".*/\" -aP ./ $REMOTE_DIR
```
