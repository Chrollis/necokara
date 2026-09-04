# Contributing to Necokara

Thanks for considering contributing! This project is currently maintained by **Chrollis**, so the process is lightweight but still follows good practices to keep the history clean and readable.

## Legal Agreement (Contributor License)

By submitting a pull request (PR) to this repository, you agree to the following terms:

- Your contribution is licensed under the **Necokara License 1.0** (see [LICENSE](LICENSE)).
- You grant **Chrollis** the permanent, irrevocable, worldwide, royalty-free right to relicense your contribution under the terms of Section 3(e) of the Necokara License (which permits GPLv3) or under separate commercial terms, if and when Chrollis deems necessary, without further notification to you.
- You warrant that you have the legal right to make this contribution and grant this license.

> _In short: submitting code means you're okay with it being distributed under the project's current license, and you trust the maintainer (Chrollis) to handle relicensing (e.g., to GPLv3 or for commercial use) in the future._

## Branch Strategy (recommended)

- **`main`** – the stable, released version. Please don't commit directly to it.
- **`develop`** – the integration branch for ongoing work. If you're adding a new feature or fixing a bug, branch off `develop`.
- **`feature/*`** – name your branch after the feature (e.g., `feature/add-login`).

If you prefer, you can also work directly on `main` for tiny fixes, but branching is always safer.

## How to Contribute

1. **Fork** the repository (or clone it directly if you have write access).
2. **Create a new branch** from `develop` (or `main` if no `develop` exists):
   ```bash
   git checkout -b feature/your-feature-name
   ```
3. **Write your code**. Feel free to run any linter or tests if they exist, but it's not mandatory.
4. **Commit your changes** – this is the most important part! Please follow the **Conventional Commits** format (explained below).
5. **Push** your branch to your remote fork:
   ```bash
   git push origin feature/your-feature-name
   ```
6. **Open a Pull Request** against the `develop` branch (or `main` if that's the only branch). In the PR description, briefly explain what you changed and why.

## Commit Message Guidelines (Conventional Commits)

We use the **Conventional Commits** specification to make the history automatically meaningful and easy to read.

### Format

```
<type>(<optional scope>): <subject>
```

- **`type`** – category of the change (see list below).
- **`scope`** – optional, specifies the module or file affected (e.g., `auth`, `api`, `docs`).
- **`subject`** – a short, imperative description of what the commit does (no period at the end).

### Allowed Types

| Type       | When to use                                    |
| ---------- | ---------------------------------------------- |
| `feat`     | A new feature for the user                     |
| `fix`      | A bug fix                                      |
| `docs`     | Documentation changes (README, comments, etc.) |
| `style`    | Code style/formatting (no logic change)        |
| `refactor` | Code restructuring without changing behavior   |
| `perf`     | Performance improvement                        |
| `test`     | Adding or modifying tests                      |
| `chore`    | Build, tooling, or dependency updates          |

### Examples

- `feat(auth): add password reset endpoint`
- `fix(api): handle null response in user profile`
- `docs: update installation instructions`
- `refactor(logger): simplify error logging`
- `chore(deps): upgrade lodash to 4.17.21`

### Breaking Changes

If your commit introduces a breaking change (not backward‑compatible), add an exclamation mark after the type/scope and include `BREAKING CHANGE:` in the footer:

```
feat(api)!: change response format for /users
```

Then explain the migration in the commit body.

## Pull Request & Merge

- **Chrollis** will review your PR as soon as possible.
- There is no strict CI or coverage requirement, but I may ask for adjustments.
- Once approved, I will merge your PR – usually with **Squash and Merge** to keep a clean history. This means your PR title should already be a good conventional commit message.

## Questions?

Feel free to open an issue or ping **Chrollis** directly. Thank you for your help!
