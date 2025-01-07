{
  lib,
  stdenv,
  statix,
  flakeSource,
}:
stdenv.mkDerivation {
  name = "statix-check";
  src = flakeSource;
  dontBuild = true;
  doCheck = true;
  buildInputs = [statix];

  checkPhase = ''
    statix check "$src"
  '';

  installPhase = ''
    touch "$out"
  '';

  meta = with lib; {
    description = "Lint nix files";
    licenses = licenses.gpl2Only;
    maintainers = [maintainers.aftix];
    platforms = platforms.linux;
  };
}
