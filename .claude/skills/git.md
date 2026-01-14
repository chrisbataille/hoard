# Git Workflow Skill

Guided git workflow for the Hoard project following our branching strategy.

## Triggers
- `/git` - Interactive git workflow helper
- When user wants to create a feature, fix, hotfix, or release branch

## Workflow

When this skill is invoked, ask the user what they want to do:

### Options

1. **Start a new feature** (`feature/*`)
2. **Start a bug fix** (`fix/*`)
3. **Create a hotfix** (`hotfix/*`) - for production emergencies
4. **Prepare a release** (`release/*`)
5. **Finish current branch** - merge workflow
6. **Check branch status** - show current state

---

## Branch Workflows

### 1. New Feature

```bash
# Ensure develop is up to date
git checkout develop
git pull origin develop

# Create feature branch
git checkout -b feature/<name>
```

**Ask user for:** Short feature name (kebab-case, e.g., `add-nix-source`)

### 2. Bug Fix

```bash
# Ensure develop is up to date
git checkout develop
git pull origin develop

# Create fix branch
git checkout -b fix/<name>
```

**Ask user for:** Short description of the bug (kebab-case, e.g., `history-parsing-crash`)

### 3. Hotfix (Production Emergency)

```bash
# Branch from main
git checkout main
git pull origin main

# Create hotfix branch
git checkout -b hotfix/<name>
```

**Ask user for:** Short description (kebab-case, e.g., `db-corruption`)

**Remind user:** Hotfixes must be merged to BOTH `main` AND `develop`

### 4. Prepare Release

```bash
# Ensure develop is up to date
git checkout develop
git pull origin develop

# Create release branch
git checkout -b release/v<version>
```

**Ask user for:** Version number (e.g., `0.2.0`)

**Checklist for release:**
- [ ] Update version in `Cargo.toml`
- [ ] Update CHANGELOG.md
- [ ] Run full test suite: `cargo test`
- [ ] Run clippy: `cargo clippy`
- [ ] Build release: `cargo build --release`

### 5. Finish Current Branch

Based on current branch type:

**Feature/Fix branch → develop:**
```bash
git checkout develop
git pull origin develop
git merge --no-ff <branch>
# Or create PR for squash merge
```

**Release branch → main:**
```bash
git checkout main
git pull origin main
git merge --no-ff release/v<version>
git tag -a v<version> -m "Release v<version>"
git push origin main --tags

# Back-merge to develop
git checkout develop
git merge main
git push origin develop
```

**Hotfix branch → main AND develop:**
```bash
# To main
git checkout main
git merge --no-ff hotfix/<name>
git tag -a v<version> -m "Hotfix v<version>"
git push origin main --tags

# To develop
git checkout develop
git merge main
git push origin develop
```

### 6. Check Branch Status

Show:
- Current branch
- Uncommitted changes
- Commits ahead/behind
- Recent commits on current branch

```bash
git status
git log --oneline -5
git branch -vv
```

---

## Commit Message Format

Remind users of the format:
```
type(scope): short description
```

Types: `feat`, `fix`, `docs`, `refactor`, `test`, `chore`, `perf`

---

## Pre-PR Checklist

Before finishing any branch, verify:
- [ ] `cargo test` passes
- [ ] `cargo clippy` has no warnings
- [ ] `cargo fmt` applied
- [ ] Fish completions updated if CLI changed

---

## Quick Reference

| Action | Command |
|--------|---------|
| New feature | `git checkout develop && git checkout -b feature/name` |
| New fix | `git checkout develop && git checkout -b fix/name` |
| New hotfix | `git checkout main && git checkout -b hotfix/name` |
| New release | `git checkout develop && git checkout -b release/vX.Y.Z` |
| Tag release | `git tag -a vX.Y.Z -m "Release vX.Y.Z"` |
