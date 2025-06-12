## Release Flow

1. Bump version in `Cargo.toml`
2. Run `cargo build` to bump version in `Cargo.lock`
3. Update version in `README.md`
  1. Version in `Latest Version`
  2. Version in the installation snippets
4. Commit the changes.
5. Create a tag: `git tag -m 'vX.Y.Z' vX.Y.Z`
6. Push commit and tag: `git push -u --tags origin master`
7. Go to the GitHub Actions `dist` workflow: https://github.com/RagnarLab/litemon/actions/workflows/dist.yml
8. Click on `Run workflow`
9. Select the created tag (e.g., `vX.Y.Z`) and click `Run workflow`
10. Go to `Releases`: https://github.com/RagnarLab/litemon/releases
11. Publish the created draft release
12. Profit!
