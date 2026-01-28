{
  description = "A feature rich customizable TUI app launcher written in Rust";
  
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }: flake-utils.lib.eachDefaultSystem (system:
    let
      pkgs = nixpkgs.legacyPackages.${system};
    in
    {
      packages.default = pkgs.callPackage ./default.nix { };
      apps.default = {
        type = "app";
        program = "${self.packages.${system}.default}/bin/flare";
      };
      devShells.default = pkgs.mkShell {
        nativeBuildInputs = with pkgs; [
          rustc
          cargo
          rust-analyzer
          pkg-config
        ];
      };
    }
  );
}