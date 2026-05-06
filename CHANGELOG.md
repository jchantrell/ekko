# Changelog

## [0.13.1](https://github.com/jchantrell/ekko/compare/v0.13.0...v0.13.1) (2026-05-06)


### Bug Fixes

* daemon respawns shim on any request failure, not just heavy ones ([a635523](https://github.com/jchantrell/ekko/commit/a635523d14701e369ef420f89f07e469a7f0b814))
* limit queue to 1 concurrent worker for local inference ([60b705d](https://github.com/jchantrell/ekko/commit/60b705d56febad61f42f336d97543df6a2662744))
* single graphiti group for unified entity resolution ([a68d8bf](https://github.com/jchantrell/ekko/commit/a68d8bff04b6e291b6954e235949cb6919854711))

## [0.13.0](https://github.com/jchantrell/ekko/compare/v0.12.1...v0.13.0) (2026-05-05)


### Features

* add anthropic provider support ([809036a](https://github.com/jchantrell/ekko/commit/809036a7e7e38d0a73a68becf8d98b1d2aef5120))
* add groups discovery and frontmatter ([24cc3cf](https://github.com/jchantrell/ekko/commit/24cc3cff79e56390241da00b06d7431e67e0afdf))


### Bug Fixes

* auto-start Neo4j on boot via podman persistence ([a6187ed](https://github.com/jchantrell/ekko/commit/a6187ed1662293056deceeab616ca788618af845))
* resolve clippy warnings failing CI ([60bda80](https://github.com/jchantrell/ekko/commit/60bda80e9a2fbaeb83a06cf42239390cbf31ce88))

## [0.12.1](https://github.com/jchantrell/ekko/compare/v0.12.0...v0.12.1) (2026-03-23)


### Bug Fixes

* auto-respawn shim when it dies during request handling ([0565d79](https://github.com/jchantrell/ekko/commit/0565d797e672f7446dbc26ba1b6e357f7822d2b7))
* non-blocking queue/health/status when shim is busy ([e23a6ed](https://github.com/jchantrell/ekko/commit/e23a6edb222a608cb533bf94790300e65aa14855))

## [0.12.0](https://github.com/jchantrell/ekko/compare/v0.11.0...v0.12.0) (2026-03-20)


### Features

* add ekko queue CLI command ([86fb596](https://github.com/jchantrell/ekko/commit/86fb596b7ca8ac4f27c063a9ef91ade456d6e6ff))
* scope ekko sync to current project by default, add --all flag ([7b2a864](https://github.com/jchantrell/ekko/commit/7b2a86469618f28a144685d3b07d4abfad92b003))


### Bug Fixes

* prevent concurrent daemon starts with flock ([88f32f2](https://github.com/jchantrell/ekko/commit/88f32f20a882beb31e700623a3842d58929f1262))

## [0.11.0](https://github.com/jchantrell/ekko/compare/v0.10.1...v0.11.0) (2026-03-20)


### Features

* add daemon server with socket multiplexer and lifecycle management ([5aeed89](https://github.com/jchantrell/ekko/commit/5aeed89e0cff7ae5b691427d19662c6b71dbc7db))
* add DaemonClient for Unix socket communication ([11c989b](https://github.com/jchantrell/ekko/commit/11c989bb933e14f6e5c86e368b05b7fceaa39845))
* add periodic background sync to daemon ([4341c15](https://github.com/jchantrell/ekko/commit/4341c1525361a905fb2fa4251397cedd3267757c))
* wire all clients through daemon with auto-start and fallback ([9ab75e6](https://github.com/jchantrell/ekko/commit/9ab75e6bd0c12e5e9290d8fce2fda25f79567659))

## [0.10.1](https://github.com/jchantrell/ekko/compare/v0.10.0...v0.10.1) (2026-03-20)


### Bug Fixes

* always run container setup on init, use full image ref for podman ([0548499](https://github.com/jchantrell/ekko/commit/0548499a8a58f50e4d1175945049d2fdd5123802))

## [0.10.0](https://github.com/jchantrell/ekko/compare/v0.9.0...v0.10.0) (2026-03-20)


### Features

* migrate from FalkorDB to Neo4j ([6bdbd53](https://github.com/jchantrell/ekko/commit/6bdbd53ea34637e8f9f053a0af21299c04d531ed))


### Bug Fixes

* normalize group_id comparison in session filter ([da656e2](https://github.com/jchantrell/ekko/commit/da656e27161916bee64deb6d09101b3b1a085648))

## [0.9.0](https://github.com/jchantrell/ekko/compare/v0.8.0...v0.9.0) (2026-03-20)


### Features

* session indexer (`ekko sync`) ([492886a](https://github.com/jchantrell/ekko/commit/492886acf131820fb4417b68f4cc51d3883486a8))


### Bug Fixes

* resolve clippy collapsible_if warnings ([45798cd](https://github.com/jchantrell/ekko/commit/45798cd5c84b4361893a7eeaedecf01cf937b461))
* scope background sync to current project, cap at 50 sessions ([40bd6fa](https://github.com/jchantrell/ekko/commit/40bd6fa5bff58d8bce3fd4178940b4a4ba83df53))

## [0.8.0](https://github.com/jchantrell/ekko/compare/v0.7.1...v0.8.0) (2026-03-20)


### Features

* default to qwen3 models for LLM and embeddings ([6a6f19b](https://github.com/jchantrell/ekko/commit/6a6f19ba0e88193df76c638a5561a195dc0a2a8b))

## [0.7.1](https://github.com/jchantrell/ekko/compare/v0.7.0...v0.7.1) (2026-03-20)


### Bug Fixes

* remove queue CLI command, keep as MCP-only tool ([4244cc9](https://github.com/jchantrell/ekko/commit/4244cc92fe7a165b6047f75f7e709f345ccb202f))

## [0.7.0](https://github.com/jchantrell/ekko/compare/v0.6.0...v0.7.0) (2026-03-20)


### Features

* add queue status command and MCP tool ([ef72553](https://github.com/jchantrell/ekko/commit/ef72553ce194f479db869dae8055db042927f5e0))


### Bug Fixes

* update to latest release regardless of semver compatibility ([2a92716](https://github.com/jchantrell/ekko/commit/2a927168fd1c2f418691ccdf5212cd531656d100))

## [0.6.0](https://github.com/jchantrell/ekko/compare/v0.5.3...v0.6.0) (2026-03-19)


### Features

* auto-detect group from cwd in clear command ([918543d](https://github.com/jchantrell/ekko/commit/918543d80748fb43cb7ca8aceba8941ea5dc4a54))

## [0.5.3](https://github.com/jchantrell/ekko/compare/v0.5.2...v0.5.3) (2026-03-19)


### Bug Fixes

* drop ekko_ prefix from MCP tool names ([7fd3100](https://github.com/jchantrell/ekko/commit/7fd31005f7fa55283dd26fcb00bd96f7f57c9335))


### Reverts

* remove global memory, keep hyphen sanitization and rm subcommands ([fe8cedb](https://github.com/jchantrell/ekko/commit/fe8cedb505b4905009a4cf2d350d3fede65862f4))

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
