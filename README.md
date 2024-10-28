# Universal Entity Tracking System

UETS is a universal system that tracks entities using RFID tags. It is designed to be used in a variety of applications, such as inventory management, establishment access control, and more.

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

## 📝 TODO

### Dashboard View

1. Filter date range

### Entities View

1. Add details pane showing:
   - N entries and exits
   - Last action (whether entered or exited)
   - Timeline of actions
   - Other data
2. Adapt on operation mode change:
   - Attendance and Counter displays based on ID
   - Refrigerator and Inventory displays based on name
3. Implement search or filter by inside or outside, ID, name, location, etc.

### Timeline View

1. Search by date, id, name, location, etc
2. Filter date range
3. Use name when it exists
4. Implement auto scroll to the bottom
