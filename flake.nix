{
  description = "tree-sitter English grammar feasibility study";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";

  outputs = { self, nixpkgs }:
    let
      system = "aarch64-darwin";
      pkgs = nixpkgs.legacyPackages.${system};
    in {
      devShells.${system}.default = pkgs.mkShell {
        packages = with pkgs; [
          tree-sitter
          nodejs
          cargo
          rustc
          rustfmt
          # C compiler comes from Xcode CLT (/usr/bin/cc) on macOS.
          # Set CC if the tree-sitter CLI does not find it.
        ];
      };
    };
}
