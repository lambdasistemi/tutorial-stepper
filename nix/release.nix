# The files a release uploads: one static binary per Linux architecture,
# named with the crate version, and one checksum file that lists both.
{ pkgs
, packages
, version
}:
let
  x86 = "tutorial-stepper-${version}-x86_64-linux";
  arm = "tutorial-stepper-${version}-aarch64-linux";
in
pkgs.runCommand "tutorial-stepper-release-artifacts"
{
  nativeBuildInputs = [ pkgs.coreutils ];
}
''
  set -euo pipefail
  mkdir -p "$out"
  cp ${packages.tutorial-stepper-static}/bin/tutorial-stepper \
    "$out/${x86}"
  cp ${packages.tutorial-stepper-static-aarch64}/bin/tutorial-stepper \
    "$out/${arm}"
  chmod 755 "$out/${x86}" "$out/${arm}"
  (
    cd "$out"
    sha256sum ${x86} ${arm} | sort -k 2 > SHA256SUMS
  )
''
