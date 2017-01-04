#!/usr/bin/env sh
cargo update && cargo build --release
mkdir target/build/
cp target/release/vulcan-test target/build/
rsync -av --progress assets target/build/ --exclude assets/build
version="v$(grep version Cargo.toml | awk '{print $3}' | tr -d '"')"
filename=vulcan-test-$version-linux-x86_64.tar
cd target/build/
rm -r *.tar.*
tar cf $filename * --remove-files
xz --compress -9e $filename --force
