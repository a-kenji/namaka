# namaka

[![release](https://img.shields.io/github/v/release/nix-community/namaka?logo=github&style=flat-square)](https://github.com/nix-community/namaka/releases)
[![version](https://img.shields.io/crates/v/namaka?logo=rust&style=flat-square)](https://crates.io/crates/namaka)
[![deps](https://deps.rs/repo/github/nix-community/namaka/status.svg?style=flat-square&compact=true)](https://deps.rs/repo/github/nix-community/namaka)
[![license](https://img.shields.io/badge/license-MPL--2.0-blue?style=flat-square)](https://www.mozilla.org/en-US/MPL/2.0)
[![ci](https://img.shields.io/github/actions/workflow/status/nix-community/namaka/ci.yml?label=ci&logo=github-actions&style=flat-square)](https://github.com/nix-community/namaka/actions/workflows/ci.yml)

Snapshot testing tool for Nix based on [haumea]

```bash
nix shell github:nix-community/namaka
namaka check # run checks
namaka review # review pending snapshots
```

![](https://user-images.githubusercontent.com/40620903/230751675-b1eb1076-bcd8-4c21-a420-f4c914716bb9.gif)

## Usage

```
Usage: namaka <COMMAND>

Commands:
  check   Wrapper around `nix flake check` to prepare snapshots for failed tests [aliases: c]
  review  Review pending snapshots and selectively accept or reject them [aliases: r]
  help    Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version
```

### [`load`](nix/load.nix)

Type: `{ flake, dir?, ... } -> { ... }`

Arguments:

- `flake` : `Path`

  Path to the Nix flake.

- (optional) `dir` : `String`

  Path to the testing directory relative to `flake`, defaults to `tests`.
  `${flake}/${dir}` is passed to [`haumea.load`]'s `src` option.

Wrapper around [`haumea.load`] to loads snapshot tests from a directory.
See [tests](tests) for how the directory should be structured.
Ignore the `_snapshots` directory, as it is automatically generated by namaka.

The rest of the arguments are passed directory to [`haumea.load`],
with the exception of `src`, which always gets overwritten (ignored).

## Related Tools

- Namaka is largely inspired by [insta](https://github.com/mitsuhiko/insta),
  a snapshot testing tool for Rust.
- Namaka is based on [haumea], which also has some testing functionalities.

[`haumea`]: https://github.com/nix-community/haumea
[`haumea.load`]: https://github.com/nix-community/haumea#load
