# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.3](https://github.com/chrisbataille/hoards/compare/v0.1.2...v0.1.3) - 2026-01-14

### Bug Fixes

- correct CLI name from hoard to hoards ([#4](https://github.com/chrisbataille/hoards/pull/4))

### Features

- add pre-commit hooks for code quality ([#5](https://github.com/chrisbataille/hoards/pull/5))

## [0.1.2] - 2025-01-14

### Added
- Shell completions command using clap_complete
- Config/dotfiles management commands (link, unlink, sync, status)
- Initial project structure

### Features
- Multi-source tool tracking (cargo, apt, pip, npm, brew, flatpak)
- Usage analytics via shell history parsing
- AI integration for categorization and discovery
- GitHub sync for stars, descriptions, topics
- Bundle management for grouping related tools
