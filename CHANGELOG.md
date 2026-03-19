# Changelog

## [0.5.2](https://github.com/jchantrell/ekko/compare/v0.5.1...v0.5.2) (2026-03-19)


### Bug Fixes

* sanitize hyphens in group_id for FalkorDB/RediSearch compatibility ([a1ab31e](https://github.com/jchantrell/ekko/commit/a1ab31e4f0128b88d1712ebff2a4f90e97d843fb))

## [0.5.1](https://github.com/jchantrell/ekko/compare/v0.5.0...v0.5.1) (2026-03-19)


### Bug Fixes

* collapse nested if to satisfy clippy ([a65d655](https://github.com/jchantrell/ekko/commit/a65d655bd865d1debd41a86c77fed2840541e4c8))

## [0.5.0](https://github.com/jchantrell/ekko/compare/v0.4.0...v0.5.0) (2026-03-19)


### Features

* add global memory support via _global group_id ([321d4a1](https://github.com/jchantrell/ekko/commit/321d4a1f03b3daea088be32a8f375a5ba44e1c7b))
* add rm subcommands for facts and episodes ([2bf3f97](https://github.com/jchantrell/ekko/commit/2bf3f975004ce10369eab25fe69a2f9a466a09c3))

## [0.4.0](https://github.com/jchantrell/ekko/compare/v0.3.0...v0.4.0) (2026-03-19)


### Features

* add CLI commands (add, ask, show, rm, nodes, episodes, clear) ([7664c69](https://github.com/jchantrell/ekko/commit/7664c69677ac06cf7d2fd44ccc4fe036bf8c328d))
* add shared client initialization helper for CLI commands ([0431aca](https://github.com/jchantrell/ekko/commit/0431aca8cb9af69be823c3a376f44e652f3d6927))
* wire CLI commands into main and clean up dead code ([047a0f4](https://github.com/jchantrell/ekko/commit/047a0f43875ab3f68fa3742ae59c8d08832f10cd))


### Bug Fixes

* handle Graphiti error responses in call_tool_json ([1d5c3dd](https://github.com/jchantrell/ekko/commit/1d5c3dd9c5fc1112dd3399359297d3755a9f2f1d))
* show full UUIDs in list output, truncate multiline node summaries ([8286df2](https://github.com/jchantrell/ekko/commit/8286df2269fa339bd1f138ce72856787aa7cfa3c))

## [0.3.0](https://github.com/jchantrell/ekko/compare/v0.2.2...v0.3.0) (2026-03-19)


### Features

* add 'ekko serve' command to start MCP server ([7cb57f7](https://github.com/jchantrell/ekko/commit/7cb57f7c7ec4158d2195f6d81678f38c5ba14aea))
* add MCP server with 6 tools over STDIO ([352e011](https://github.com/jchantrell/ekko/commit/352e01147c080dd11a7986acba02911373caaabc))
* add project detection from cwd for group_id scoping ([10e2945](https://github.com/jchantrell/ekko/commit/10e294515746159e4e710ca31eaaa86cb7b1e730))
* add rmcp, chrono, schemars, tracing-subscriber deps ([dbcae1f](https://github.com/jchantrell/ekko/commit/dbcae1f113258272f7e5060a525a0a171414c659))


### Bug Fixes

* route Graphiti LLM calls to local Ollama via OPENAI_BASE_URL ([897c4af](https://github.com/jchantrell/ekko/commit/897c4af0e0fad55c1e0a01d723458ef30903dda0))

## [0.2.2](https://github.com/jchantrell/ekko/compare/v0.2.1...v0.2.2) (2026-03-19)


### Bug Fixes

* use fully qualified image names for podman, write config before model pulls ([bab4544](https://github.com/jchantrell/ekko/commit/bab454491e5d284f7a29acd825accd32ed4bbb2f))

## [0.2.1](https://github.com/jchantrell/ekko/compare/v0.2.0...v0.2.1) (2026-03-18)


### Bug Fixes

* package binary at archive root for self-update ([b6cf661](https://github.com/jchantrell/ekko/commit/b6cf661fd21ed3783cb6f39057c05be547294366))

## [0.2.0](https://github.com/jchantrell/ekko/compare/v0.1.0...v0.2.0) (2026-03-18)


### Features

* support podman as container runtime ([ec58c63](https://github.com/jchantrell/ekko/commit/ec58c63407e1f4bbaa7c65a92771bcae38a1fa94))


### Bug Fixes

* install to ~/.local/bin, no sudo required ([b8e5225](https://github.com/jchantrell/ekko/commit/b8e5225f9b1ee0e644025545d01b9b1353d33343))
* use sudo for chmod in install script ([0f225ce](https://github.com/jchantrell/ekko/commit/0f225ce0e6e96a3cee4e04ef8da0d0d85e574183))

## 0.1.0 (2026-03-18)


### Features

* add license ([fea62c4](https://github.com/jchantrell/ekko/commit/fea62c4b4a19f68e2261d122e3644cdb69b64239))
* add self-update command and release pipeline ([077c3ae](https://github.com/jchantrell/ekko/commit/077c3ae36a52257cb8ce878bf01b5d6421641850))
* v0.1 scaffold with Graphiti client, CLI, init, and doctor ([2eadae1](https://github.com/jchantrell/ekko/commit/2eadae1f7fc69d5ddac923178602e77b62f6d3e0))


### Bug Fixes

* resolve clippy collapsible_if warning ([ae42d4d](https://github.com/jchantrell/ekko/commit/ae42d4d8b259b69ee5bfc83ee921c335b31a371f))
