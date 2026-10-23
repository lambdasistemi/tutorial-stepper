# Changelog

## [0.4.1](https://github.com/lambdasistemi/tutorial-stepper/compare/v0.4.0...v0.4.1) (2026-10-23)


### Bug Fixes

* prefer the signal's exit when the pane dies with it (issue 47) ([237c38d](https://github.com/lambdasistemi/tutorial-stepper/commit/237c38d3d864b87e4cb0e13e353c90db55f18376))
* refuse a shell without readline and kill the server on signals (issue 47) ([e342fa0](https://github.com/lambdasistemi/tutorial-stepper/commit/e342fa060935859ab2fecb2d1602741b98d779db))
* refuse a shell without readline and remove the server on signals ([94cd8f1](https://github.com/lambdasistemi/tutorial-stepper/commit/94cd8f18129a8c6ab8c5f464b142a72d70fc5aea))

## [0.4.0](https://github.com/lambdasistemi/tutorial-stepper/compare/v0.3.1...v0.4.0) (2026-10-23)


### Features

* a window's folder may be the run folder itself ([da4327d](https://github.com/lambdasistemi/tutorial-stepper/commit/da4327d6076f05d7e416328f5c77cad63f9d63b0))
* a window's folder may be the run folder itself ([0c44fd3](https://github.com/lambdasistemi/tutorial-stepper/commit/0c44fd3cd779ab3cee5c8b497554cf3e6b6c79ae))

## [0.3.1](https://github.com/lambdasistemi/tutorial-stepper/compare/v0.3.0...v0.3.1) (2026-10-23)


### Bug Fixes

* push the Homebrew formula with the token as basic credentials ([dd5fe8b](https://github.com/lambdasistemi/tutorial-stepper/commit/dd5fe8b982996629dd1670e67bc08d0ddaffd110)), closes [#40](https://github.com/lambdasistemi/tutorial-stepper/issues/40)
* serve one SVG favicon instead of five missing PNGs ([58b22d3](https://github.com/lambdasistemi/tutorial-stepper/commit/58b22d3a8e2f8dc966ef0834ff83b5c13bf5b7e8))

## [0.3.0](https://github.com/lambdasistemi/tutorial-stepper/compare/v0.2.0...v0.3.0) (2026-10-23)


### Features

* build the macOS formula in CI and publish it on release ([c3754b5](https://github.com/lambdasistemi/tutorial-stepper/commit/c3754b566f46ef527f21bf7d6035476f735cd722))
* checksum the Apple Silicon tarball and play both shells ([6ecc177](https://github.com/lambdasistemi/tutorial-stepper/commit/6ecc177c4355104e492527c4cb2ceb86a87bce73))
* choose settings before the first page ([b23c99d](https://github.com/lambdasistemi/tutorial-stepper/commit/b23c99df192a76f13c875df43cae8d596ad718b6))
* choose settings before the first page ([5a27e09](https://github.com/lambdasistemi/tutorial-stepper/commit/5a27e097e9d0730628aa57d330e697e016b104c9))
* fold the explanations away ([9664867](https://github.com/lambdasistemi/tutorial-stepper/commit/9664867299e504c3cd559a0d33e8ee2fadcc5ca2))
* fold the explanations away ([8cad255](https://github.com/lambdasistemi/tutorial-stepper/commit/8cad255dcf9431c35978c85fc95f34f886ba0ead))
* macOS binary and Homebrew formula ([b036e50](https://github.com/lambdasistemi/tutorial-stepper/commit/b036e5068efb0c7b995d8b186a20894d672643c5))
* push a command to the prompt before it runs ([15d119c](https://github.com/lambdasistemi/tutorial-stepper/commit/15d119c087f30b03068ef045a417a1b481953ef9))
* push a command to the prompt before it runs ([835c5c9](https://github.com/lambdasistemi/tutorial-stepper/commit/835c5c92d3507dcdd54fafa6da61e7df711b6d64))
* render the Homebrew formula from a version and a checksum ([7d9b6b8](https://github.com/lambdasistemi/tutorial-stepper/commit/7d9b6b873cdbeecbdc4eee03d00788c836fba58f))
* run an observation at any time ([9ba1fb3](https://github.com/lambdasistemi/tutorial-stepper/commit/9ba1fb3581a1cf86bf3d647c6df5fb631e66fe8c))
* run an observation at any time ([d3152ca](https://github.com/lambdasistemi/tutorial-stepper/commit/d3152cabcf5ccba908a98c0efc574121aa749845))
* ship static Linux binaries for x86_64 and arm64 ([e451986](https://github.com/lambdasistemi/tutorial-stepper/commit/e451986c23ea73413edd1352d61c28c6ff72e572))
* ship static Linux binaries for x86_64 and arm64 ([fbf7145](https://github.com/lambdasistemi/tutorial-stepper/commit/fbf71457861bba9eb12f3be09fc9e03411531f10))


### Bug Fixes

* keep a shell off the new settings choice ([7999d5a](https://github.com/lambdasistemi/tutorial-stepper/commit/7999d5adcb24c9e354b96edc74a09c8710b3386c))
* put the Homebrew binary where the formula installs it ([44db25c](https://github.com/lambdasistemi/tutorial-stepper/commit/44db25cd0ffb109c077b49dd08b73e4aa5d2fb87))
* retry running a test program that is still busy ([89b2e93](https://github.com/lambdasistemi/tutorial-stepper/commit/89b2e93c38d98a380c8c6cefa732b723afad7c11)), closes [#39](https://github.com/lambdasistemi/tutorial-stepper/issues/39)

## [0.2.0](https://github.com/lambdasistemi/tutorial-stepper/compare/v0.1.0...v0.2.0) (2026-10-23)


### Features

* check declared programs before the first command ([f803526](https://github.com/lambdasistemi/tutorial-stepper/commit/f80352680bd1bba4d5ef0210ea54b6800589587c))
* keep the run directory when --run is given ([8bc1c07](https://github.com/lambdasistemi/tutorial-stepper/commit/8bc1c077a72c1d12f0ff188ff123de502be89c41))
* observations in the unattended run ([4bc9335](https://github.com/lambdasistemi/tutorial-stepper/commit/4bc9335f1b4d575db869c28aa41d4554cc9a5f14))
* play a one-window tutorial unattended ([63a327e](https://github.com/lambdasistemi/tutorial-stepper/commit/63a327e38ef85cc9e41e87c0763b700fff7aa02f))
* play a one-window tutorial unattended ([089925f](https://github.com/lambdasistemi/tutorial-stepper/commit/089925f761664ad513f0e4ccef6c99e4174ff2f6))
* play a tutorial in the windows it declares ([7bb4e3c](https://github.com/lambdasistemi/tutorial-stepper/commit/7bb4e3c3a9b2f545d9d6ebf9cd0c067d28bdda81))
* play a tutorial page by page in the terminal ([f452030](https://github.com/lambdasistemi/tutorial-stepper/commit/f4520300e474c6263e13af5c2e1f572824cafd9d))
* play a tutorial page by page in the terminal ([9f86adb](https://github.com/lambdasistemi/tutorial-stepper/commit/9f86adbcec4acb2122d416e5f883ea21a89b8b28))
* play every tested path from a clean run folder ([0cadb10](https://github.com/lambdasistemi/tutorial-stepper/commit/0cadb102579c06c2e17346e98bc8f09cbab892e2))
* play observations where they appear and again at the end of the path ([6d24975](https://github.com/lambdasistemi/tutorial-stepper/commit/6d24975e9ac17076b510d2a000c4a8ad88a64e2c))
* play one tested path and load its values ([610d615](https://github.com/lambdasistemi/tutorial-stepper/commit/610d61569a0e534e4368403cd546580e45ee77e4))
* play unattended tutorials in zsh ([a90c611](https://github.com/lambdasistemi/tutorial-stepper/commit/a90c611cf3224c09dc732f93b26775c56a8d7ebe))
* play unattended tutorials in zsh ([890a38a](https://github.com/lambdasistemi/tutorial-stepper/commit/890a38ab4b8af42906a5bf1626e928cb756dda8c))
* print a tutorial's commands as one script ([66ed781](https://github.com/lambdasistemi/tutorial-stepper/commit/66ed7817a905e109e75d242aceced2cc77d1bff6))
* print a tutorial's commands as one script ([32600ba](https://github.com/lambdasistemi/tutorial-stepper/commit/32600baa27bb205d2cfe65891498220926fc9c79))
* print one tested path with script --path ([3321da3](https://github.com/lambdasistemi/tutorial-stepper/commit/3321da39e86d2bee0a4225f709d40761a923f7bb))
* run the set-up script when --run-setup is given ([b7068fb](https://github.com/lambdasistemi/tutorial-stepper/commit/b7068fb8e31f51435641d1b93e77826592d0dc9a))
* settings and tested paths in the unattended run ([d97e6c9](https://github.com/lambdasistemi/tutorial-stepper/commit/d97e6c9bef226324822a730133049009dc217961))
* several windows, the set-up script and the needs check ([754746a](https://github.com/lambdasistemi/tutorial-stepper/commit/754746a762b7841867acb56e443a2d6cf18ecb05))


### Bug Fixes

* compare the version tests with the crate's own version ([aadeefa](https://github.com/lambdasistemi/tutorial-stepper/commit/aadeefa23b4912b36fb51f98e028d87a239767aa)), closes [#29](https://github.com/lambdasistemi/tutorial-stepper/issues/29)
* refuse the set-up script when the user is root ([4e9ecb7](https://github.com/lambdasistemi/tutorial-stepper/commit/4e9ecb740aecb7b1ffc66b3671545c6011d8f01b))
* show the window name as the prompt and the typed command ([6c04330](https://github.com/lambdasistemi/tutorial-stepper/commit/6c043305e4f2c1f7d3adade13b1065bd2d57c5b4))

## 0.1.0 (2026-10-22)


### Features

* add the tutorial-stepper binary ([2ead093](https://github.com/lambdasistemi/tutorial-stepper/commit/2ead0930392f0cc0df91d0bd494bc44d96619751))
* check a tutorial directory against tutorial.toml ([c9ac6f0](https://github.com/lambdasistemi/tutorial-stepper/commit/c9ac6f02b86e582c0b17e6f0b3c74e8db60cebe6))
* check a tutorial's format ([ce2b15d](https://github.com/lambdasistemi/tutorial-stepper/commit/ce2b15d3e16155920ed42699d51dc36693d3d391))
* read fence attributes from the info string ([acc624b](https://github.com/lambdasistemi/tutorial-stepper/commit/acc624b616ee08c2c7711debf5c40745cc84478b))


### Bug Fixes

* name tutorial.toml when the index is missing ([31dbd11](https://github.com/lambdasistemi/tutorial-stepper/commit/31dbd111fc12da6a79f627ad8f7eb52429ee7709))
