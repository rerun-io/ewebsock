- [ ] Update `CHANGELOG.md`
- [ ] Bump version number
- [ ] `./check.sh`
- [ ] `./example_app/build_web.sh --open`
- [ ] `git commit -m 'Release 0.x.0 - summary'`
- [ ] `cargo publish -p ewebsock`
- [ ] `git tag -a 0.x.0 -m 'Release 0.x.0 - summary'`
* [ ] `git pull --tags && git tag -d latest && git tag -a latest -m 'Latest release' && git push --tags origin latest --force`
* [ ] `git push && git push --tags`
- [ ] Check that CI is green
- [ ] Do a GitHub release: https://github.com/rerun-io/ewebsock/releases/new
- [ ] Post on Twitter
