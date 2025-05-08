<div align="center">
    <h1>Tempus</h1>
    <strong>Minimalist Terminal Timer with Style</strong>
    <img src= "./.github/assets/demo.gif alt="Tempus Demo">
</div>

## Features

- **Progress Visualization** - 4 visual themes with smooth transitions
- **Preset Timers** - Quick access to common timers (pomodoro, breaks, etc.)
- **Notification Options** - Desktop and sound alerts when your timer completes
- **Minimal Interface** - Clean and elegant design that stays out of your way

## Installation

The recommended way of installing Tempus is with [Nix](https://nixos.org/).
Simply install it with `nix profile install github:NotAShelf/tempus` or add it
as a flake input.

Or build from source:

```bash
git clone https://github.com/notashelf/tempus.git
cd tempus
cargo build --release
```

## Usage

```bash
# Basic timer - 5 minutes
tempus 5m

# Pomodoro preset with notifications
tempus -p pomodoro -n

# Custom timer with a name and rainbow theme
tempus 30m -n "Meditation" -t rainbow

# Short break without sound notification
tempus -p short-break --bell=false
```

## Progress Bar Themes

Tempus comes with four default themes:

- **Gradient** - Colors shift from green to yellow to red (default)
- **Rainbow** - Colorful display with blocks in rainbow colors
- **Pulse** - Animated pulsing effect with cyan/blue colors
- **Simple** - Classic monochrome style for distraction-free focus

## Command Line Options

| Option          | Description                              |
| --------------- | ---------------------------------------- |
| `-n, --name`    | Give your timer a name                   |
| `-v, --verbose` | Show more detailed output                |
| `-t, --theme`   | Choose progress bar theme                |
| `-p, --preset`  | Use a preset duration                    |
| `-b, --bell`    | Enable/disable terminal bell sound       |
| `-N, --notify`  | Send desktop notification when completed |

### Available Presets

- `pomodoro` - 25 minutes
- `short-break` - 5 minutes
- `long-break` - 15 minutes
- `tea` - 3 minutes
- `coffee` - 4 minutes

## Building & Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

This project is licensed under the MIT License - see the [LICENSE file](LICENSE)
for details.
