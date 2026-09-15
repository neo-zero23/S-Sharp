# Maintainer: S# Language Project
pkgname=ssharp
pkgver=0.1.1
pkgrel=1
pkgdesc="S# - lenguaje de programación educativo inspirado en Scratch"
arch=('x86_64')
url="https://github.com/neo-zero23/S-Sharp"
license=('MIT')
depends=()
makedepends=('cargo' 'rust')
# NOTE: upstream repo is "S-Sharp", so the extracted dir is S-Sharp-$pkgver.
_realname=S-Sharp
source=("$pkgname-$pkgver.tar.gz::https://github.com/neo-zero23/S-Sharp/archive/v$pkgver.tar.gz")
sha256sums=('SKIP')

build() {
  cd "$srcdir/$_realname-$pkgver"
  cargo build --release --locked
}

package() {
  cd "$srcdir/$_realname-$pkgver"
  install -Dm755 "target/release/ssharp" "$pkgdir/usr/bin/ssharp"
  install -Dm644 "README.md" "$pkgdir/usr/share/doc/$pkgname/README.md"
  install -Dm644 "LICENSE" "$pkgdir/usr/share/licenses/$pkgname/LICENSE"
  install -Dm644 "ssharp.desktop" "$pkgdir/usr/share/applications/ssharp.desktop"
  install -Dm644 examples/*.ssharp -t "$pkgdir/usr/share/doc/$pkgname/examples/"
}
