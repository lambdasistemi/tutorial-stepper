# The documentation site and the check that reads it. The crate's
# source filter does not include these pages, so a docs edit does not
# rebuild the binary.
{ pkgs }:
let
  root = toString ../.;
  docsSrc = pkgs.lib.cleanSourceWith {
    src = ../.;
    filter = path: type:
      let
        rel = pkgs.lib.removePrefix "${root}/" (toString path);
      in
      toString path == root
      || rel == "mkdocs.yml"
      || rel == "docs"
      || pkgs.lib.hasPrefix "docs/" rel
      || rel == "tools"
      || rel == "tools/check_site.py";
  };

  mkdocs-terminal = pkgs.python3Packages.buildPythonPackage rec {
    pname = "mkdocs-terminal";
    version = "4.8.0";
    format = "wheel";
    src = pkgs.fetchurl {
      url = "https://files.pythonhosted.org/packages/cd/21/7eb37356eeeaa87be873c806ea84794b3b81285e49a6c7c4a250c66729a6/mkdocs_terminal-${version}-py3-none-any.whl";
      sha256 = "86af80cc7152aa61e9058db0a84eefae56e769222e81eb5291ad87e89d3f2922";
    };
    doCheck = false;
    dependencies = with pkgs.python3Packages; [
      jinja2
      markdown
      mkdocs
      pygments
      pymdown-extensions
    ];
  };

  pythonEnv = pkgs.python3.withPackages (ps: [
    ps.mkdocs
    ps.pymdown-extensions
    mkdocs-terminal
  ]);

  site = pkgs.runCommand "tutorial-stepper-site"
    {
      nativeBuildInputs = [ pythonEnv pkgs.glibcLocales ];
      LANG = "C.UTF-8";
      LC_ALL = "C.UTF-8";
    }
    ''
      set -euo pipefail
      cp -R ${docsSrc}/. site-src
      chmod -R u+w site-src
      cd site-src
      mkdocs build --strict --site-dir "$out"
    '';

  check = pkgs.runCommand "tutorial-stepper-documentation"
    {
      nativeBuildInputs = [ pkgs.python3 pkgs.glibcLocales ];
      LANG = "C.UTF-8";
      LC_ALL = "C.UTF-8";
    }
    ''
      set -euo pipefail
      python3 ${docsSrc}/tools/check_site.py ${site}
      touch "$out"
    '';
in
{
  inherit site check;
}
