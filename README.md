# Universal Entity Tracking System

UETS is a universal system that tracks entities using RFID tags. It is designed to be used in a variety of applications, such as inventory management, establishment access control, and more.

## ✨ Features

### Report Generation

- Filter by date range, entity ID, name, location, etc.
- Visualize data in a graph.
- Create report about depleting stocks, overflows, etc.
- Export as PDF or XLSX via QR code.

### Real-time Monitoring

- View entities entering and exiting in real-time.
- View real time statistics and graphs.
- Notify on stock depletion or capacity overflow.

### Easy Data Handling

- Pre-input data via spreadsheet files.
- Support for BPSU CEA's QRifying system and national ID QR codes.

### Automation

- Automatically control IoT devices, such as lights, doors, etc. based on entity count data.

## 🕹️ Operation Modes

### Counter

This is used for entities that don't have any specific data (e.g., mall entry counter).

### Attendance

This is used for tracking attendance, whereas unauthorized entities are not allowed to enter (e.g., classroom, meeting room, school gate, establishment entry).

### Parking

This is used for tracking parking spaces and vehicles (e.g., parking lot).

This is useful for tracking how long a vehicle has been parked and whether it is authorized to park.

### Inventory

This is used for entities that have lifetime, location, and quantity (e.g., stock room, department store, medicine storage).

### Refrigerator

This is used for entities that have lifetime and quantity (e.g., food storage).

This is experimental as it is labor-intensive to tag entities with this kind.

## 🏗️ Building and Running

1. Set up a toolbox container.
   - Run, `toolbox create --distro ubuntu --release 24.04`
2. Set up Rust via `rustup`.
   - Optionally, install `rust-analyzer` via `rustup component add rust-analyzer`.
3. Run `./setup` to install the required dependencies.
4. Run `./run` to build and run the project.

## 🔧 Setting up Touchscreen Rotation

1. Add the following to `/etc/udev/rules.d/99-calibration.rules`:

```
ATTRS{name}=="wch.cn USB2IIC_CTP_CONTROL", ENV{LIBINPUT_CALIBRATION_MATRIX}="0 1 0 -1 0 1"
```

2. Reboot the system.

## 🔃 Syncing code to the Pi

```sh
rsync --filter=':- .gitignore' --exclude \".*/\" -aP ./ $REMOTE_DIR
```
