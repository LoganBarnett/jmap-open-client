{
  description = "Open, extensible JMAP client for Rust";
  inputs = {
    # LLM: Do NOT change this URL unless explicitly directed. This is the
    # correct format for nixpkgs stable (25.11 is correct, not nixos-25.11).
    nixpkgs.url = "github:NixOS/nixpkgs/25.11";
    rust-overlay.url = "github:oxalica/rust-overlay";
    crane.url = "github:ipetkov/crane";
    changelog-roller.url = "github:LoganBarnett/changelog-roller";
    foundation.url = "github:LoganBarnett/rust-template";
    foundation.inputs.nixpkgs.follows = "nixpkgs";
    org-fmt.url = "github:LoganBarnett/org-fmt";
    org-fmt.inputs.nixpkgs.follows = "nixpkgs";
    org-fmt.inputs.rust-overlay.follows = "rust-overlay";
    org-fmt.inputs.crane.follows = "crane";
  };

  outputs = {
    self,
    nixpkgs,
    rust-overlay,
    crane,
    changelog-roller,
    foundation,
    org-fmt,
  } @ inputs: let
    forAllSystems =
      nixpkgs.lib.genAttrs nixpkgs.lib.systems.flakeExposed;
    perSystem = forAllSystems (system: let
      pkgs = import nixpkgs {
        inherit system;
        overlays = [(import rust-overlay)];
      };
      craneLib =
        (crane.mkLib pkgs).overrideToolchain
        (p: p.rust-bin.stable.latest.default);
    in
      foundation.lib.mkRustProject {
        inherit self pkgs craneLib;
        name = "jmap-open-client";
        crates = {
          # CRATE:cli:begin
          cli = {
            name = "jmap-open-client-cli";
            binary = "jmap-open-client-cli";
            description = "CLI application";
          };
          # CRATE:cli:end
          # CRATE_ENTRIES

          # Note: The 'lib' crate is not included here as it doesn't
          # produce a binary.
        };
        extraDevPackages = [
          pkgs.cargo-sweep
          pkgs.jq
          # Unified formatter
          pkgs.treefmt
          pkgs.alejandra
          pkgs.prettier
          pkgs.just
          changelog-roller.packages.${system}.default
          org-fmt.packages.${system}.default
        ];
        shellHook = ''
          echo "jmap-open-client development environment"
          echo ""
          echo "Available Cargo packages (use 'cargo build -p <name>'):"
          cargo metadata --no-deps --format-version 1 2>/dev/null | \
            jq -r '.packages[].name' | \
            sort | \
            sed 's/^/  • /' || echo "  Run 'cargo init' to get started"
        '';
      });
  in {
    devShells =
      nixpkgs.lib.mapAttrs (_: p: {default = p.devShell;}) perSystem;
    packages = nixpkgs.lib.mapAttrs (_: p: p.packages) perSystem;
    apps = nixpkgs.lib.mapAttrs (_: p: p.apps) perSystem;
  };
}
