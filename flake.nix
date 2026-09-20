{
  description = "tree-sitter English grammar feasibility study";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";

  outputs = { self, nixpkgs }:
    let
      system = "aarch64-darwin";
      pkgs = nixpkgs.legacyPackages.${system};
    in {
      # Dev shell: `nix develop`, then:
      #   npx -y tree-sitter-cli@0.27.0 generate  # regenerate the parser
      #   cargo test --workspace                  # binding, AST and corpus tests
      #   cargo fmt --check && cargo clippy --workspace --all-targets
      # NOTE: the shell's tree-sitter CLI is 0.26.9 while package.json pins
      # 0.27.0. Generating with 0.26.9 only churns src/tree_sitter/array.h,
      # so always generate with the pinned 0.27. The CLI test/parse commands
      # cannot link the Rust external scanner at any version; the corpus
      # runs under cargo instead (see README).
      devShells.${system}.default = pkgs.mkShell {
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
}
