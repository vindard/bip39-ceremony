{
  description = "Inspectable physical-entropy to BIP-39 ceremony";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs = { self, nixpkgs, ... }:
    let
      systems = [
        "aarch64-darwin"
        "aarch64-linux"
        "x86_64-darwin"
        "x86_64-linux"
      ];
      forAllSystems = nixpkgs.lib.genAttrs systems;
      packageFor = pkgs: rustPlatform:
        rustPlatform.buildRustPackage {
          pname = "bip39-ceremony";
          version = "0.1.0";
          src = nixpkgs.lib.fileset.toSource {
            root = ./.;
            fileset = nixpkgs.lib.fileset.gitTracked ./.;
          };

          cargoLock.lockFile = ./Cargo.lock;
          cargoBuildFlags = [ "--package" "bip39-ceremony-tui" ];
          cargoTestFlags = [ "--workspace" "--all-targets" "--all-features" ];
          strictDeps = true;

          meta = {
            description = "Inspectable physical-entropy to BIP-39 ceremony";
            license = nixpkgs.lib.licenses.mit;
            mainProgram = "bip39-ceremony";
          };
        };
    in
    {
      packages = forAllSystems (system:
        let
          pkgs = nixpkgs.legacyPackages.${system};
          dynamic = packageFor pkgs pkgs.rustPlatform;
        in
        {
          default = dynamic;
          gnu = dynamic;
        } // nixpkgs.lib.optionalAttrs pkgs.stdenv.isLinux {
          musl = packageFor pkgs pkgs.pkgsStatic.rustPlatform;
        });

      checks = forAllSystems (system:
        let
          pkgs = nixpkgs.legacyPackages.${system};
          smoke = name: package: pkgs.runCommand name
            { nativeBuildInputs = [ pkgs.python3Minimal ]; }
            ''
              python ${./scripts/pty-smoke.py} ${package}/bin/bip39-ceremony
              touch $out
            '';
        in
        {
          release-gnu = self.packages.${system}.gnu;
          smoke-gnu = smoke "bip39-ceremony-smoke-gnu" self.packages.${system}.gnu;
        } // nixpkgs.lib.optionalAttrs pkgs.stdenv.isLinux {
          release-musl = self.packages.${system}.musl;
          smoke-musl = smoke "bip39-ceremony-smoke-musl" self.packages.${system}.musl;
        });

      devShells = forAllSystems (system:
        let
          pkgs = nixpkgs.legacyPackages.${system};
        in
        {
          default = pkgs.mkShell {
            packages = with pkgs; [
              cargo
              cargo-audit
              cargo-deny
              clippy
              diffoscopeMinimal
              file
              just
              python3Minimal
              ripgrep
              rustc
              rustfmt
            ] ++ lib.optionals stdenv.isLinux [ guix ];

            RUST_BACKTRACE = "1";
          };
        });
    };
}
