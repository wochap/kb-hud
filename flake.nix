{
  description = "kb-hud development shell";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-26.05";
  };

  outputs = { self, nixpkgs }:
    let
      system = "x86_64-linux";
      pkgs = nixpkgs.legacyPackages.${system};
    in
    {
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
