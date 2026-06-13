# Maintainer: your-name <your@email>
pkgname=thempl
pkgver=0.1.0
pkgrel=1
pkgdesc='Jinja2+YAML config templater with TUI'
arch=(x86_64 aarch64)
url='https://github.com/xable/thempl'
license=(MIT)
depends=(gcc-libs)
makedepends=(cargo)
source=("$pkgname::git+https://github.com/xable/thempl.git")
sha256sums=('SKIP')

pkgver() {
    cd "$srcdir/$pkgname"
    grep '^version' Cargo.toml | cut -d'"' -f2
}

build() {
    cd "$srcdir/$pkgname"
    cargo build --release --locked
}

check() {
    cd "$srcdir/$pkgname"
    cargo test --release --locked
}

package() {
    cd "$srcdir/$pkgname"
    install -Dm755 target/release/$pkgname "$pkgdir/usr/bin/$pkgname"
    install -Dm644 README.md "$pkgdir/usr/share/doc/$pkgname/README.md"
    install -Dm644 LICENSE "$pkgdir/usr/share/licenses/$pkgname/LICENSE"
}
