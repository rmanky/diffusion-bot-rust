{
  description = "A development environment with Rust and flyctl";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
  };

  outputs =
    { self, nixpkgs, ... }:
    let
      pkgs = import nixpkgs { system = "x86_64-linux"; };
    in
    {
      devShells.x86_64-linux.default = pkgs.mkShell {
        packages = [
          pkgs.rustc
          pkgs.cargo
          pkgs.flyctl
          pkgs.pkg-config
          pkgs.openssl
          pkgs.cargo-watch
          pkgs.rust-analyzer
          pkgs.cmake
        ];
      };
    };
}
