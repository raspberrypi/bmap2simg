# bmap2simg

A utility that converts disk images with block map (bmap) sparse information to the Android sparse image format. The tool reads bmap files that describe which blocks of a disk image contain actual data, then creates an Android sparse image that efficiently stores only the mapped blocks whilst preserving the original image structure. This is particularly useful for creating compact, flashable images from larger disk images that contain significant amounts of empty space. Many fastboot implementations directly support sparse images.

## Building for Debian

This project is packaged as a native Debian package and uses Debian's `librust-*` packages for dependencies rather than downloading crates from crates.io, ensuring an isolated and reproducible build environment.

### Build Instructions

Install build dependencies:
```bash
sudo mk-build-deps -i
```

Build the package:
```bash
debuild -b -uc -us
```

The built `.deb` package will be created in the parent directory.

Cargo may be used directly in support of incremental builds by setting the
following environment variables:
```bash
export CARGO_HOME=debian/cargo_home
export CARGO_REGISTRY=debian/cargo_registry
```

### Dependencies

All Rust dependencies are satisfied by Debian's `librust-*` packages as specified in `debian/control`. This ensures the build is completely isolated from external package registries and uses only packages that have been reviewed and packaged by Debian maintainers.
