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
        "crates/english-pos/weights/*" # generated weights (compact JSON)
        "crates/english-pos/lexicon/*" # generated wordlists, byte-stable
        "grammar.js" # upstream style, no JS formatter configured
        "binding.gyp"
        "test/corpus/*" # fixtures must stay byte-stable
        "examples/*" # snapshots must stay byte-stable
        "crates/english-pos-train/data/*" # gold oracle labels, byte-stable
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
        "scripts/harmonize-ud.mjs" # node transform, no JS formatter configured
        "scripts/*.py" # dev-time oracle scripts, no Python formatter configured
        "scripts/*.txt" # pinned book-ID list + md5 manifest, must stay byte-stable
      ];
      programs.alejandra.enable = true; # nix
      programs.jsonfmt.enable = true; # json
      programs.mdformat.enable = true; # markdown
      programs.rustfmt.enable = true; # rust
      programs.taplo.enable = true; # toml
      programs.shfmt.enable = true; # shell (scripts/fetch-ud.sh)
    };
  };
}
