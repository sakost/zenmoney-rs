# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.4.1](https://github.com/sakost/zenmoney-rs/compare/v0.4.0...v0.4.1) - 2026-08-15

### Fixed

- make Merchant.user nullable (Option<UserId>)

## [0.4.0](https://github.com/sakost/zenmoney-rs/compare/v0.3.0...v0.4.0) - 2026-08-02

### Added

- *(client)* Report JSON path in response deserialization errors

### Fixed

- *(models)* Accept day and week payoff intervals

### Other

- Remove stale changelog_config from release-plz config
- *(deps)* Update aws-lc-sys and rustls-webpki
