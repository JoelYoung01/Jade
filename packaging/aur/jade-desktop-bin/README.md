# AUR: `jade-desktop-bin`

Binary AUR package that installs the Linux `.deb` published by Jade’s GitHub Releases workflow.

On Arch / EndeavourOS, prefer updating with yay (the desktop app can open Konsole with this command):

```bash
yay -S --needed jade-desktop-bin
```

Do not use the AppImage in-app updater against a pacman-owned install — Jade detects AUR installs and routes updates through yay instead.

## One-time AUR setup

1. Create an account at <https://aur.archlinux.org> and add your SSH public key.
2. Claim the package name (first push creates it):

```bash
git clone ssh://aur@aur.archlinux.org/jade-desktop-bin.git
cd jade-desktop-bin
cp /path/to/Jade/packaging/aur/jade-desktop-bin/PKGBUILD .
cp /path/to/Jade/packaging/aur/jade-desktop-bin/jade-desktop-bin.install .
```

3. After a real GitHub Release exists for `pkgver`, set a real `sha256sums_x86_64` (replace `SKIP`), then:

```bash
makepkg --printsrcinfo > .SRCINFO
makepkg -si   # local install test on EndeavourOS
git add PKGBUILD .SRCINFO jade-desktop-bin.install
git commit -m "jade-desktop-bin $pkgver-$pkgrel"
git push
```

## After each Jade release

From this repo’s template (keep it in sync with the AUR clone):

1. Bump `pkgver` in `PKGBUILD` to the released version (e.g. `0.2.0`).
2. Compute the checksum:

```bash
pkgver=0.2.0
curl -L -o "Jade_${pkgver}_amd64.deb" \
  "https://github.com/JoelYoung01/Jade/releases/download/v${pkgver}/Jade_${pkgver}_amd64.deb"
sha256sum "Jade_${pkgver}_amd64.deb"
```

3. Put that hash in `sha256sums_x86_64`.
4. In the AUR git clone: copy files, `makepkg --printsrcinfo > .SRCINFO`, commit, push.

If the `.deb` asset name from `tauri-action` differs, update the `source_x86_64` URL to match the release asset exactly.
