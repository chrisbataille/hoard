# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.3](https://github.com/chrisbataille/hoard/compare/v0.1.2...v0.1.3) - 2026-01-14

### Bug Fixes

- simplify release-plz config to use defaults
- remove invalid filter_commits from release-plz config
- correct git skill directory structure

### Documentation

- add git branching workflow and contributing guidelines

### Features

- CLI simplification and CI/CD automation ([#1](https://github.com/chrisbataille/hoard/pull/1))

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
