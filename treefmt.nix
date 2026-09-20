# See available programs from https://github.com/numtide/treefmt-nix/tree/main/programs
{inputs, ...}: {
  imports = [inputs.treefmt-nix.flakeModule];
  perSystem = {
    pkgs,
    lib,
    ...
  }: {
    treefmt = {
      settings.on-unmatched = "fatal"; # Ensure 100% coverage
      settings.global.excludes = [
        ".editorconfig"
        "LICENSE"
        "src/*" # generated parser
        "grammar.js" # upstream style, no JS formatter configured
        "binding.gyp"
        "test/corpus/*" # fixtures must stay byte-stable
        "examples/*" # snapshots must stay byte-stable
        "bindings/c/*" # generated
        "bindings/go/*" # generated
        "bindings/node/*" # generated
        "bindings/python/*" # generated
        "bindings/swift/*" # generated
        "setup.py" # generated
        "go.mod" # generated
        "Package.swift" # generated
        "Makefile" # generated
        "CMakeLists.txt" # generated
      ];
      programs.alejandra.enable = true; # nix
      programs.jsonfmt.enable = true; # json
      programs.mdformat.enable = true; # markdown
      programs.rustfmt.enable = true; # rust
      programs.taplo.enable = true; # toml
    };
  };
}
