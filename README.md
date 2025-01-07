# N-Body Simulation

A gravitational N-body simulation implemented in Rust, supporting native and WebAssembly targets. The simulation uses the Barnes-Hut algorithm for force calculations and real-time visualization using OpenGL/WebGL.

![N-Body Simulation](https://raw.githubusercontent.com/robmdunn/nbody-rs/main/nbody.png)

## Features

- N-body gravitational simulation using the Barnes-Hut algorithm
- Real-time visualization with OpenGL (native) and WebGL (web)
- Parallel computation support for native builds using Rayon
- Interactive parameter adjustment through GUI
- Support for both native desktop and WebAssembly targets
- State saving and loading for simulation checkpoints
- Adaptive viewport scaling and fixed-scale viewing options

## Build Requirements

### Common Requirements
- Rust toolchain (install via [rustup](https://rustup.rs/))

### WASM Build Additional Requirements
- wasm-pack (`cargo install wasm-pack`)

## Building and Running

### Native Build

1. Clone the repository:
```bash
git clone https://github.com/robmdunn/nbody-rs.git
cd nbody-rs
```

2. Build and run in release mode:
```bash
cargo run -p nbody-native --release
```

The native version supports various command-line arguments:

```bash
nbody --help
```

Example with custom parameters:
```bash
cargo run -p nbody-native --release -- -n 10000 --mass 2000 --spin 0.05
```

### WebAssembly Build

1. Build the WASM package:
```bash
wasm-pack build crates/nbody-wasm --target web --out-dir ../../www/pkg
```

2. Serve the web directory:
```bash
# Using Python's built-in server
python3 -m http.server 8080 --directory www/ 
```

3. Open your browser and navigate to `http://localhost:8080`

## Usage

### Native Application

The native application provides command-line options for configuration:

```
Options:
  -n, --n-bodies <N_BODIES>      Number of bodies to simulate [default: 1000]
  -m, --mass <MASS>              Mass for randomly distributed bodies [default: 2000]
  -g, --g <G>                    Gravitational constant [default: 0.0000000000667384]
  -d, --dt <TIMESTEP>            Simulation timestep [default: 0.1]
  -f, --sf <SOFTENING>           Softening factor to prevent singularities [default: 0.005]
  -s, --spin <SPIN>              Initial spin factor for random distribution [default: 0.05]
      --mz <MZERO>               Mass of central body [default: 10000000]
  -t, --tr <TREE_RATIO>          Tree ratio threshold for Barnes-Hut approximation [default: 3]
  -r, --resume <INPUT_FILE>      Input file to resume simulation from
  -o, --output <OUTPUT_FILE>     Output file to save simulation state
      --nsteps <WRITE_INTERVAL>  Interval (in steps) between writing output [default: 100]
      --no-graphics              Disable graphics
      --width <WIDTH>            Window width [default: 800]
      --height <HEIGHT>          Window height [default: 800]
  -p, --point-size <POINT_SIZE>  Point size for rendering bodies [default: 2]
      --fixed-scale              Use fixed scale view instead of following particles
  -h, --help                     Print help
  -V, --version                  Print version
```

### Web Interface

The web interface provides a control panel for adjusting simulation parameters in real-time:

- Number of Bodies: Particle count in the simulation
- Mass: Mass of distributed bodies
- Gravitational Constant: Strength of gravitational force
- Timestep: Simulation time increment
- Softening Factor: Prevents numerical instabilities
- Initial Spin: Angular momentum of initial distribution
- Central Mass: Mass of the central body
- Tree Ratio: Barnes-Hut approximation threshold
- Point Size: Size of rendered particles
- Fixed Scale: Toggle between adaptive and fixed viewport

## License

This project is licensed under the MIT License - see the LICENSE file for details.
