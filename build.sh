#!/usr/bin/env sh
cargo update && cargo build --release
mkdir target/build/
cp target/release/vulcan-test target/build/
rsync -av --progress assets target/build/ --exclude assets/build
cd target/build/
tar cf vulcan-test-linux-x86_64.tar * --remove-files
xz --compress -9e vulcan-test-linux-x86_64.tar --force
