# Karere binary package for Arch Linux (AUR)

This packaging is independent of the custom-css downstream build. It builds
unmodified stable tagged releases from https://github.com/tobagin/karere
inside a fresh Arch Linux container, bundles the release's pinned proprietary-
codec CEF runtime, and publishes an installation tree archive to this fork's
GitHub Releases. The AUR package then installs that **prebuilt** archive.

## Layout

- PKGBUILD: AUR recipe for karere-bin (no compilation).
- cef_manifest.py: extracts the CEF URL and expected SHA256 from the checked-out
  upstream Flatpak manifest; cross-checks the CEF version with download-cef.sh.
- build-arch.sh: creates the binary and resource archive in Arch Linux.
- .github/workflows/karere-bin.yml: validates and publishes GitHub and AUR releases.

The archive includes /usr/bin/karere, the native GTK application and icons,
a licence, schemas, desktop metadata, and all matching CEF runtime files under
/usr/lib/karere/cef. The launcher retains the working-directory convention used
by the existing Arch package.

## Release process

GitHub Actions checks upstream stable tags daily, excluding CEF asset tags
and prereleases. It builds the latest stable tag when there is no existing
GitHub binary release with that version and package release number.

The release tag is karere-bin-vVERSION-PKGREL, and the downloadable archive is
karere-VERSION-PKGREL-linux-x86_64.tar.zst with an adjacent SHA256 file.
Every build re-verifies the upstream proprietary-codec CEF download against the
official Flatpak manifest. The AUR PKGBUILD must have the actual archive SHA256,
not the all-zero placeholder committed in this repository.

Publishing to AUR is guarded behind a successful build and release verification.
The workflow uses the existing repository secret AUR_SSH_PRIVATE_KEY. The key's
matching public key must have AUR write access to the new karere-bin package.
The AUR host SSH key fingerprint is checked against the pinned fingerprint from
the existing custom-css workflow.

First-time publishing requires the package name to be free and the AUR account
to be permitted to create it. The workflow does not create or configure your
GitHub Actions secret. It never changes the existing karere-custom-css package.

## Manual run and packaging-only updates

Run the GitHub Action with workflow_dispatch to select a stable upstream tag and
optionally set pkgrel (default 1). Bump pkgrel manually for rebuilt binaries
of the *same* upstream version. An existing GitHub release is never overwritten,
and an AUR release is never downgraded. AUR source checksums must always be
regenerated from the actual released archive.

Pull requests validate the full Arch binary build but do not publish to GitHub
Releases or AUR. Scheduled and manual publishing is limited to main.

## Known limitations

- Only x86_64 is packaged.
- Native/non-Flatpak packaging is not officially supported by Karere upstream;
  validate actual startup on Arch before relying on it.
- Prebuilt binaries are produced by this fork, not by Karere upstream.
- The upstream 4.4 series includes normal user.css support but not the
  downstream live-reload patches.
