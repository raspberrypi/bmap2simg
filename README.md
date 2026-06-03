# bmap2simg

A utility that converts disk images with block map (bmap) sparse information to the Android sparse image format. The tool reads bmap files that describe which blocks of a disk image contain actual data, then creates an Android sparse image that efficiently stores only the mapped blocks whilst preserving the original image structure. This is particularly useful for creating compact, flashable images from larger disk images that contain significant amounts of empty space. Many fastboot implementations directly support sparse images.

## Building for Debian

This project is packaged as a native Debian package and uses Debian's `librust-*` packages for dependencies rather than downloading crates from crates.io, ensuring an isolated and reproducible build environment.

### Package Build Instructions

Install build tools:
```bash
sudo apt install git-buildpackage sbuild
```

Build the package in a dedicated chroot:
```bash
gbp buildpackage
```

To build for a specific CPU architecture:
```bash
gbp buildpackage --arch=arm64
```
Note: this is not cross-compilation — a chroot is prepared for the architecture
you request and qemu-user emulation is used (`qemu-user` package) if this does
not match your host architecture; arch-native tools are always used for builds.

The build artifacts can be found in the ../build-area directory.

### Incremental Builds

Incremental builds, used during development, require build-deps to be installed
on the host:
```bash
sudo mk-build-deps --install --remove
```

These can be later removed (if they're no longer required) with:
```bash
sudo apt-get remove --auto-remove bmap2simg-build-deps
```

Prepare a package-specific cargo home and registry:
```bash
dpkg-buildpackage -T dev-configure -nc
```

Cargo can then be used with the CARGO\_HOME environment variable set, e.g.:
```bash
CARGO_HOME=debian/cargo_home cargo check
```

### Dependencies

All Rust dependencies are satisfied by Debian's `librust-*` packages as specified in `debian/control`. This ensures the build is completely isolated from external package registries and uses only packages that have been reviewed and packaged by Debian maintainers.
