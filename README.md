# Universal Entity Tracking System

UETS is a universal system that tracks entities using RFID tags. It is designed to be used in a variety of applications, such as inventory management, establishment access control, and more.

## ✨ Features

### Report Generation

- Filter by date range, entity ID, name, location, etc.
- Visualize data in a graph.
- Export as PDF or XLSX via QR code.

### Real-time Monitoring

- View entities entering and exiting in real-time.
- View real time statistics and graphs.

## 🕹️ Operation Modes

### Counter

This is used for entities that don't have any specific data (e.g., mall entry counter).

### Inventory

This is used for entities that have lifetime, location, and quantity (e.g., stock room, department store, medicine storage).

### Refrigerator

This is used for entities that have lifetime and quantity (e.g., food storage).

This is experimental as it is labor-intensive to tag entities with this kind.

### Attendance

This is used for tracking attendance, whereas unauthorized entities are not allowed to enter (e.g., classroom, meeting room, school gate, establishment entry).

### Parking

This is used for tracking parking spaces and vehicles (e.g., parking lot).

This is useful for tracking how long a vehicle has been parked and whether it is authorized to park.

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

### Stocks & Entities View

1. Add details pane showing:
   - N entries and exits
   - Last action (whether entered or exited)
   - Timeline of actions
   - Other data
2. Implement search or filter by inside or outside, ID, name, location, etc.

### Timeline View

1. Search by date, id, name, location, etc
2. Filter date range
