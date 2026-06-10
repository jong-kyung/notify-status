# Changesets

This project uses [Changesets](https://github.com/changesets/changesets) to manage release notes and version bumps.

For changes that should be released, run:

```sh
pnpm changeset
```

The release workflow creates a `chore: update versions` pull request from pending changesets. Merging that pull request publishes the package from GitHub Actions.
