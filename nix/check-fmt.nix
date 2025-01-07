{
  lib,
  stdenv,
  flakeSource,
  alejandra,
}:
stdenv.mkDerivation {
  name = "alejandra-check";
  src = flakeSource;

  dontBuild = true;
  doCheck = true;
  buildInputs = [alejandra];

  checkPhase = ''
    alejandra -c "$src"
  '';

  installPhase = ''
    touch "$out"
  '';

  meta = with lib; {
    description = "Test that nix files are formatted";
    licenses = licenses.gpl2Only;
    maintainers = [maintainers.aftix];
    platforms = platforms.linux;
  };
}
