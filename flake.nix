{
  description = "Inspectable physical-entropy to BIP-39 ceremony";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    coldcard = {
      url = "github:Coldcard/firmware";
      flake = false;
    };
    seedsigner = {
      url = "github:SeedSigner/seedsigner";
      flake = false;
    };
    embit = {
      # SeedSigner requirements.txt pins embit 0.8.0.
      url = "github:diybitcoinhardware/embit/v0.8.0";
      flake = false;
    };
    iancoleman = {
      url = "github:iancoleman/bip39";
      flake = false;
    };
  };

  outputs = { self, nixpkgs, coldcard, seedsigner, embit, iancoleman, ... }:
    let
      systems = [
        "aarch64-darwin"
        "aarch64-linux"
        "x86_64-darwin"
        "x86_64-linux"
      ];
      forAllSystems = nixpkgs.lib.genAttrs systems;
      projectSource = nixpkgs.lib.fileset.toSource {
        root = ./.;
        fileset = nixpkgs.lib.fileset.gitTracked ./.;
      };
      packageFor = pkgs: rustPlatform:
        rustPlatform.buildRustPackage {
          pname = "bip39-ceremony";
          version = "0.1.0";
          src = projectSource;

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
          referenceDriver = pkgs.rustPlatform.buildRustPackage {
            pname = "bip39-ceremony-reference-driver";
            version = "0.1.0";
            src = projectSource;
            cargoLock.lockFile = ./Cargo.lock;
            cargoBuildFlags = [
              "--package"
              "bip39-ceremony-reference-driver"
            ];
            doCheck = false;
            strictDeps = true;
          };
        in
        {
          default = dynamic;
          gnu = dynamic;
          reference-driver = referenceDriver;
        } // nixpkgs.lib.optionalAttrs pkgs.stdenv.isLinux {
          musl = packageFor pkgs pkgs.pkgsStatic.rustPlatform;
        });

      checks = forAllSystems (system:
        let
          pkgs = nixpkgs.legacyPackages.${system};
          coreDriver = "${self.packages.${system}.reference-driver}/bin/bip39-ceremony-reference-driver";
          pythonCheck = name: path: extraInputs: extraPythonPath: command:
            pkgs.runCommand name
              { nativeBuildInputs = [ pkgs.python3Minimal ] ++ extraInputs; }
              ''
                export PYTHONPATH=${./tests/references/harness}${extraPythonPath}
                python ${path} --core ${coreDriver} ${command}
                touch $out
              '';
          referenceHarness = pythonCheck
            "reference-harness"
            ./tests/references/harness/check.py
            [ ]
            ""
            "";
          referenceColdcard = pythonCheck
            "reference-coldcard"
            ./tests/references/coldcard/check.py
            [ ]
            ""
            "--source ${coldcard}";
          referenceSeedSigner = pythonCheck
            "reference-seedsigner"
            ./tests/references/seedsigner/check.py
            [ ]
            ""
            "--source ${seedsigner} --embit ${embit}";
          ianPythonPath = ":${./tests/references/iancoleman}";
          ianArguments = "--node ${pkgs.nodejs}/bin/node --runner ${./tests/references/iancoleman/runner.js} --source ${iancoleman}";
          referenceIanBip39 = pythonCheck
            "reference-iancoleman-bip39"
            ./tests/references/iancoleman/bip39.py
            [ pkgs.nodejs ]
            ianPythonPath
            ianArguments;
          referenceIanLegacyDice = pythonCheck
            "reference-iancoleman-legacy-dice"
            ./tests/references/iancoleman/legacy_dice.py
            [ pkgs.nodejs ]
            ianPythonPath
            ianArguments;
          referenceImplementationChecks = {
            reference-coldcard = referenceColdcard;
            reference-seedsigner = referenceSeedSigner;
            reference-iancoleman-bip39 = referenceIanBip39;
            reference-iancoleman-legacy-dice = referenceIanLegacyDice;
          };
          referenceChecks = referenceImplementationChecks // {
            reference-harness = referenceHarness;
          };
          smoke = name: package: pkgs.runCommand name
            { nativeBuildInputs = [ pkgs.python3Minimal ]; }
            ''
              python ${./scripts/pty-smoke.py} ${package}/bin/bip39-ceremony
              touch $out
            '';
        in
        referenceChecks // {
          reference-implementations = pkgs.linkFarm "reference-implementations" referenceImplementationChecks;
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
