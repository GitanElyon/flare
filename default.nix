{
  pkgs ? import <nixpkgs> {}
}:

pkgs.rustPlatform.buildRustPackage rec {
  pname = "flare";
  version = "0.1.0";
  src = ./.;
  cargoLock = {
    lockFile = ./Cargo.lock;
  };
  nativeBuildInputs = [ pkgs.pkg-config ];
  buildInputs = [ ];

  meta = with pkgs.lib; {
    description = "A feature-rich customizable TUI app launcher written in Rust";
    license = licenses.mit;
    maintainers = with maintainers; [ gitanelyon ];
  };
}
