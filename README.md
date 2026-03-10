# dlk-gamedev

Experiments for learning Vulkan, graphics programming, and game development.

This repository currently is a Wavefront OBJ model viewer.  
It currently builds an executable capable of loading and displaying `.obj` files.

This project is primarily intended as a sandbox for exploring graphics concepts and Vulkan workflows.

---

## Features

- Load and view **Wavefront `.obj` models**
- Lightweight viewer executable
- Built with Rust
- Uses Vulkan-based rendering

More features will be added as the project evolves.

---

## Repository Structure

```
sandbox/   Rust project containing the OBJ viewer
```

---

## Dependencies

To build this project you will need:

1. **Cargo** (Rust package manager)
2. **glslc** (GLSL shader compiler)

### Install Cargo

Follow the instructions on the Rust website:

https://rust-lang.org/

### Install glslc

On Ubuntu or Debian:

```bash
sudo apt install glslc
```

---

## Building

From the repository root:

```bash
cargo build
```

---

## Installing (Ubuntu / Debian)

This project can be packaged using `cargo-deb`.

```bash
cd sandbox
cargo deb
```

If `cargo deb` is not installed:

```bash
cargo install cargo-deb
```

---

## Usage

Open an OBJ model:

```bash
dlk-objviewer model.obj
```

---

## Nemo File Manager Integration

You can add a **right-click action** in the Nemo file manager to open `.obj` files with `dlk-objviewer`.

Create the following file:

```
~/.local/share/nemo/actions/view_obj.nemo_action
```

With the following contents:

```
[Nemo Action]
Name=View obj model
Comment=Open obj model with dlk-objviewer
Exec=dlk-objviewer %F -f +z -u +y -r +x
Icon-Name=applications-graphics
Selection=s
Extensions=obj;
```

If your `.obj` files use a different coordinate system convention, you may want to modify the flags:

```
-f +z -u +y -r +x
```

to match your coordinate system. 
If you don't want to derive missing normals just set --derive-normals to false. 
If you want to set the mouse sensitivity just set --mouse-sensitivity to whatever you prefer. 

---

## Controls

Press 'o' to enter object mode and 'c' to enter camera mode.
While in camera mode, moving your mouse will rotate the camera.
While in object mode, moving your mouse will rotate the object.

Click on the application window to grab the mouse for rotations.
Press escape to free the mouse.

E, S, D, F, SPACE, and CTRL will move the camera while in camera mode.

---

## Roadmap

Planned experiments and improvements:

- Improved camera controls
- Materials and textures
- Rendering improvements
- Additional model formats

---

## License

This project is licensed under the **Apache 2.0 License**.

---

## Notes

This repository is primarily a learning project.  
Expect frequent changes as new graphics programming concepts are explored.
