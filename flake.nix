{
  description = "kb-hud: keyboard HUD overlay for the Chocofi split keyboard";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs?rev=0ad6f47ea4fe188f4bc8f0380f93ae8523337c6c"; # nixos-26.05 (10 jul 2026)
    bun2nix = {
      url = "github:nix-community/bun2nix/2.0.8";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      self,
      nixpkgs,
      bun2nix,
    }:
    let
      system = "x86_64-linux";
      pkgs = import nixpkgs {
        inherit system;
        overlays = [ bun2nix.overlays.default ];
      };
      lib = nixpkgs.lib;
      version = (lib.importTOML ./src-tauri/Cargo.toml).package.version;
    in
    {
      packages.${system}.default = pkgs.rustPlatform.buildRustPackage {
        pname = "kb-hud";
        inherit version;
        src = self;

        cargoRoot = "src-tauri";
        buildAndTestSubdir = "src-tauri";
        cargoLock.lockFile = ./src-tauri/Cargo.lock;

        bunDeps = pkgs.bun2nix.fetchBunDeps {
          bunNix = ./bun.nix;
        };

        # Only use the bun2nix hook for dependency setup (node_modules);
        # the build/check/install phases belong to cargo's hooks.
        dontUseBunBuild = true;
        dontUseBunCheck = true;
        dontUseBunInstall = true;
        doCheck = false;

        # Bun's isolated linker can hang indefinitely while reconstructing the
        # offline node_modules tree. This is a single-package application, so
        # the simpler hoisted layout is sufficient and substantially faster.
        bunInstallFlags = [
          "--linker=hoisted"
          "--ignore-scripts"
        ];

        nativeBuildInputs = [
          pkgs.bun2nix.hook
          pkgs.pkg-config
          pkgs.wrapGAppsHook3
        ];

        buildInputs = with pkgs; [
          glib
          gtk3
          librsvg
          webkitgtk_4_1
          libayatana-appindicator
        ];

        # Frontend build (tauri.conf.json beforeBuildCommand); the bun2nix hook
        # has already populated node_modules from bunDeps at this point. The
        # dist/ must exist before cargo builds: tauri embeds the frontend
        # assets at compile time.
        preBuild = ''
          bun run build
        '';

        # libappindicator-sys dlopens the tray library at runtime;
        # wrapGAppsHook does not add buildInputs to LD_LIBRARY_PATH.
        preFixup = ''
          gappsWrapperArgs+=(
            --prefix LD_LIBRARY_PATH : "${lib.makeLibraryPath [ pkgs.libayatana-appindicator ]}"
          )
        '';

        meta = {
          description = "Transparent HUD overlay that visualizes the Chocofi split keyboard in real time";
          license = lib.licenses.mit;
          mainProgram = "kb-hud";
          platforms = [ system ];
        };
      };

      apps.${system}.default = {
        type = "app";
        program = "${self.packages.${system}.default}/bin/kb-hud";
      };

      devShells.${system}.default = pkgs.mkShell {
        packages = with pkgs; [
          cargo
          clippy
          rustc
          rustfmt
          bun
          pkg-config
          webkitgtk_4_1
          gtk3
          glib
          librsvg
          openssl
          libayatana-appindicator
        ];

        shellHook = ''
          export XDG_DATA_DIRS="$(echo ${pkgs.gsettings-desktop-schemas}/share/gsettings-desktop-schemas-*):$(echo ${pkgs.glib}/share/gsettings-schemas/*):$XDG_DATA_DIRS"
          export LD_LIBRARY_PATH="${pkgs.libayatana-appindicator}/lib''${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
        '';
      };
    };
}
