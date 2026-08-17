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
    krux = {
      url = "github:selfcustody/krux/v26.08.0";
      flake = false;
    };
    bitbox02 = {
      url = "github:BitBoxSwiss/bitbox02-firmware/firmware/v9.26.4";
      flake = false;
    };
    bitbox-bip39 = {
      # Matches bitbox02-firmware's Cargo.lock git revision.
      url = "github:BitBoxSwiss/rust-bip39/d69f68c837ee7962a26619316fb7a725e2e8d44c";
      flake = false;
    };
    keystone-legacy = {
      url = "github:KeystoneHQ/Keystone-cold-app/34e638fa57aed6a54051f9fe065d501c3e129581";
      flake = false;
    };
    jade = {
      url = "github:Blockstream/Jade/1.0.40";
      flake = false;
    };
    bitcoinlib = {
      url = "github:RooSoft/bitcoinlib/a998a61caad66d074772ec4a10ba5268aa65ca40";
      flake = false;
    };
    bluewallet = {
      url = "github:BlueWallet/BlueWallet/8.0.1";
      flake = false;
    };
    bluewallet-bignumber = {
      # Matches BlueWallet's package.json dependency.
      url = "github:MikeMcl/bignumber.js/v9.3.1";
      flake = false;
    };
    jade-libwally = {
      # Matches Jade's components/libwally-core/upstream gitlink.
      url = "github:ElementsProject/libwally-core/43b97bed2e5b6347a909bfd1113242528826a8a2";
      flake = false;
    };
    jade-secp256k1 = {
      # Matches libwally-core's src/secp256k1 gitlink.
      url = "github:BlockstreamResearch/secp256k1-zkp/6152622613fdf1c5af6f31f74c427c4e9ee120ce";
      flake = false;
    };
  };

  outputs = { self, nixpkgs, coldcard, seedsigner, embit, iancoleman, krux, bitbox02, bitbox-bip39, keystone-legacy, jade, jade-libwally, jade-secp256k1, bitcoinlib, bluewallet, bluewallet-bignumber, ... }:
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
          bitboxAdapterSource = pkgs.runCommand "bitbox-lastword-adapter-source"
            { nativeBuildInputs = [ pkgs.python3Minimal ]; }
            ''
              python ${./tests/references/bitbox/extract.py} \
                --source ${bitbox02} \
                --bip39 ${bitbox-bip39} \
                --output $out
              cp ${./tests/references/bitbox/Cargo.lock} $out/Cargo.lock
            '';
          bitboxAdapter = pkgs.rustPlatform.buildRustPackage {
            pname = "bitbox-lastword-adapter";
            version = "0.1.0";
            src = bitboxAdapterSource;
            cargoHash = "sha256-4Z+AaXMkQzcxREFCTYHjKyA4sDzvm4E5rmdgQpidQMs=";
            doCheck = false;
            strictDeps = true;
          };
          keystoneAdapter = pkgs.runCommand "keystone-legacy-adapter"
            { nativeBuildInputs = [ pkgs.jdk_headless pkgs.python3Minimal ]; }
            ''
              mkdir -p source $out
              python ${./tests/references/keystone/extract.py} \
                --source ${keystone-legacy} \
                --output source
              javac -d $out source/*.java
            '';
          jadeLibwally = pkgs.stdenv.mkDerivation {
            pname = "jade-libwally-core";
            version = "1.5.3";
            src = jade-libwally;
            nativeBuildInputs = [
              pkgs.autoconf
              pkgs.automake
              pkgs.libtool
              pkgs.pkg-config
              pkgs.python311
              pkgs.python311Packages.setuptools
            ];
            postUnpack = ''
              chmod -R u+w "$sourceRoot"
              mkdir -p "$sourceRoot/src/secp256k1"
              cp -R ${jade-secp256k1}/. "$sourceRoot/src/secp256k1/"
              chmod -R u+w "$sourceRoot"
            '';
            preConfigure = ''
              export SETUPTOOLS_USE_DISTUTILS=local
              ./tools/autogen.sh
            '';
            configureFlags = [
              "--disable-elements"
              "--disable-tests"
              "--disable-shared"
            ];
            enableParallelBuilding = true;
          };
          jadeAdapter = pkgs.runCommand "jade-final-word-adapter"
            {
              nativeBuildInputs = [ pkgs.pkg-config pkgs.python3Minimal pkgs.stdenv.cc ];
              buildInputs = [ jadeLibwally ];
            }
            ''
              mkdir -p $out/bin
              python ${./tests/references/jade/extract.py} \
                --source ${jade} \
                --output adapter.c
              $CC -Wno-format-security adapter.c \
                $(pkg-config --cflags --libs wallycore) -lsecp256k1 \
                -o $out/bin/jade-final-word-adapter
            '';
          bluewalletAdapter = pkgs.runCommand "bluewallet-entropy-adapter"
            { nativeBuildInputs = [ pkgs.python3Minimal pkgs.esbuild ]; }
            ''
              mkdir -p $out
              python ${./tests/references/bluewallet/extract.py} \
                --source ${bluewallet} \
                --output adapter.ts
              esbuild --format=cjs --platform=node adapter.ts --outfile=$out/adapter.js
            '';
          iancolemanDiceAdapter = pkgs.runCommand "iancoleman-dice-adapter"
            { nativeBuildInputs = [ pkgs.python3Minimal ]; }
            ''
              mkdir -p $out
              python ${./tests/references/iancoleman/dice-extract.py} \
                --source ${iancoleman} \
                --output $out/adapter.js
            '';
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
          referenceKrux = pythonCheck
            "reference-krux"
            ./tests/references/krux/check.py
            [ ]
            ""
            "--source ${krux}";
          referenceBitBox = pythonCheck
            "reference-bitbox-checksum"
            ./tests/references/bitbox/check.py
            [ ]
            ""
            "--adapter ${bitboxAdapter}/bin/bitbox-lastword-adapter";
          referenceKeystone = pythonCheck
            "reference-keystone-legacy"
            ./tests/references/keystone/check.py
            [ ]
            ""
            "--java ${pkgs.jdk_headless}/bin/java --classes ${keystoneAdapter}";
          referenceJade = pythonCheck
            "reference-jade-checksum"
            ./tests/references/jade/check.py
            [ ]
            ""
            "--adapter ${jadeAdapter}/bin/jade-final-word-adapter";
          ianPythonPath = ":${./tests/references/iancoleman}";
          ianArguments = "--node ${pkgs.nodejs}/bin/node --runner ${./tests/references/iancoleman/runner.js} --source ${iancoleman}";
          referenceIanBip39 = pythonCheck
            "reference-iancoleman-bip39"
            ./tests/references/iancoleman/bip39.py
            [ pkgs.nodejs ]
            ianPythonPath
            ianArguments;
          referenceBitcoinLib = pythonCheck
            "reference-bitcoinlib-base6"
            ./tests/references/bitcoinlib/check.py
            [ pkgs.beam_minimal.packages.erlang.elixir ]
            ""
            "--elixir ${pkgs.beam_minimal.packages.erlang.elixir}/bin/elixir --adapter ${./tests/references/bitcoinlib/adapter.exs} --source ${bitcoinlib}";
          referenceBlueWallet = pythonCheck
            "reference-bluewallet-bitpack"
            ./tests/references/bluewallet/check.py
            [ pkgs.nodejs ]
            ""
            ("--node ${pkgs.nodejs}/bin/node"
              + " --runner ${./tests/references/bluewallet/runner.js}"
              + " --adapter ${bluewalletAdapter}/adapter.js"
              + " --bignumber ${bluewallet-bignumber}/bignumber.js");
          referenceIanDice = pythonCheck
            "reference-iancoleman-dice"
            ./tests/references/iancoleman/dice.py
            [ pkgs.nodejs ]
            ""
            ("--node ${pkgs.nodejs}/bin/node"
              + " --runner ${./tests/references/iancoleman/dice-runner.js}"
              + " --source ${iancoleman}"
              + " --adapter ${iancolemanDiceAdapter}/adapter.js");
          referenceImplementationChecks = {
            reference-coldcard = referenceColdcard;
            reference-seedsigner = referenceSeedSigner;
            reference-krux = referenceKrux;
            reference-bitbox-checksum = referenceBitBox;
            reference-keystone-legacy = referenceKeystone;
            reference-jade-checksum = referenceJade;
            reference-iancoleman-bip39 = referenceIanBip39;
            reference-bitcoinlib-base6 = referenceBitcoinLib;
            reference-bluewallet-bitpack = referenceBlueWallet;
            reference-iancoleman-dice = referenceIanDice;
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
