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
      rust = pkgs.rust-bin.stable.latest.default.override {
        extensions = [
          # For rust-analyzer and others.  See
          # https://nixos.wiki/wiki/Rust#Shell.nix_example for details.
          "rust-src"
          "rust-analyzer"
          "rustfmt"
        ];
      };
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
      commonArgs = {
        src = craneLib.cleanCargoSource self;
        # Run only unit tests (--lib --bins), skip integration tests in
        # tests/ directories.  Integration tests may require external
        # services not available in the Nix sandbox.
        cargoTestExtraArgs = "--lib --bins";
      };
      rustPackages = foundation.lib.mkRustPackages {
        inherit self pkgs craneLib crates commonArgs;
      };
      packages =
        rustPackages.packages
        // {
          default =
            craneLib.buildPackage (commonArgs // {pname = "jmap-open-client";});
        };
    in {
      inherit packages;
      inherit (rustPackages) apps;
      devShell = pkgs.mkShell {
        buildInputs = [
          rust
          pkgs.cargo-sweep
          pkgs.jq
          # Unified formatter
          pkgs.treefmt
          pkgs.alejandra
          pkgs.prettier
          pkgs.just
          changelog-roller.packages.${system}.default
          org-fmt.packages.${system}.default
          # ABI baseline check used by the reusable CI workflow's `abi`
          # job.  Compares the workspace's current public API against the
          # previous version on crates.io and reports breaking changes;
          # the job then gates on an Upcoming → Breaking changelog entry
          # when a break is detected.  Provided here so contributors can
          # run `nix develop --command cargo semver-checks ...` locally
          # before opening a PR.
          #
          # `doCheck = false` skips upstream's `target_feature_*`
          # snapshot tests, which assert against snapshots recorded on
          # x86_64 and therefore fail when building on aarch64-darwin.
          # We only ship the binary, not its test suite, so disabling
          # the check phase does not affect what the workflow runs.
          (pkgs.cargo-semver-checks.overrideAttrs (_: {doCheck = false;}))
        ];
        shellHook = ''
          ${foundation.lib.cargoHuskyHookSnippet pkgs}
          echo "jmap-open-client development environment"
          echo ""
          echo "Available Cargo packages (use 'cargo build -p <name>'):"
          cargo metadata --no-deps --format-version 1 2>/dev/null | \
            jq -r '.packages[].name' | \
            sort | \
            sed 's/^/  • /' || echo "  Run 'cargo init' to get started"
        '';
      };
    });
  in {
    devShells =
      nixpkgs.lib.mapAttrs (_: p: {default = p.devShell;}) perSystem;
    packages = nixpkgs.lib.mapAttrs (_: p: p.packages) perSystem;
    apps = nixpkgs.lib.mapAttrs (_: p: p.apps) perSystem;
  };
}
