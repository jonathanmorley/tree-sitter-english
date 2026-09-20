{
  description = "tree-sitter English grammar feasibility study";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixpkgs-unstable";
    flake-parts.url = "github:hercules-ci/flake-parts";
    treefmt-nix.url = "github:numtide/treefmt-nix";
  };

  outputs = inputs @ {flake-parts, ...}:
    flake-parts.lib.mkFlake {inherit inputs;} {
      imports = [./treefmt.nix];
      systems = [
        "aarch64-darwin"
        "x86_64-linux"
        "aarch64-linux"
      ];

      perSystem = {pkgs, ...}: {
        # Dev shell: `nix develop`, then:
        #   npx -y tree-sitter-cli@0.27.0 generate  # regenerate the parser
        #   cargo test --workspace                  # binding, AST and corpus tests
        #   cargo fmt --check && cargo clippy --workspace --all-targets
        # NOTE: the shell's tree-sitter CLI is 0.26.9 while package.json pins
        # 0.27.0. Generating with 0.26.9 only churns src/tree_sitter/array.h,
        # so always generate with the pinned 0.27. The CLI test/parse commands
        # cannot link the Rust external scanner at any version; the corpus
        # runs under cargo instead (see README).
        devShells.default = pkgs.mkShell {
          packages = with pkgs; [
            tree-sitter
            nodejs
            cargo
            rustc
            rustfmt
            clippy
            # C compiler comes from Xcode CLT (/usr/bin/cc) on macOS.
            # Set CC if the tree-sitter CLI does not find it.
          ];
        };
      };
    };
}
