{
  lib,
  stdenv,
  flakeSource,
  cargo,
  cargo-sort,
}:
stdenv.mkDerivation {
  name = "cargo-sort-check";
  src = flakeSource;

  dontBuild = true;
  doCheck = true;
  buildInputs = [cargo cargo-sort];

  checkPhase = ''
    cargo sort -n -c "$src"
  '';

  installPhase = ''
    cargo sort -p "$src" > "$out"
  '';

  meta = with lib; {
    description = "Test that Cargo.toml is sorted";
    licenses = licenses.gpl2Only;
    maintainers = [maintainers.aftix];
    platforms = platforms.linux;
  };
}
