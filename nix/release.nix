# The files a release uploads: the Linux x86_64 binary and its checksum.
{ pkgs
, packages
}:
pkgs.runCommand "tutorial-stepper-release-artifacts"
{
  nativeBuildInputs = [ pkgs.coreutils ];
}
''
  set -euo pipefail
  mkdir -p "$out"
  cp ${packages.tutorial-stepper-static}/bin/tutorial-stepper \
    "$out/tutorial-stepper-linux-x86_64"
  (
    cd "$out"
    sha256sum tutorial-stepper-linux-x86_64 > SHA256SUMS
  )
''
