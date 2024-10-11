{ pkgs ? import <nixpkgs> {} }:

pkgs.mkShell {
  hardeningDisable = [ "fortify" ];
  buildInputs = with pkgs; [
    clang
  ];
  LIBCLANG_PATH = "${pkgs.libclang.lib}/lib";
}
