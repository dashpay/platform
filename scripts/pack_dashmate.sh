#!/bin/bash

set -e

cmd_usage="Usage: pack_dashmate.sh COMMAND

  COMMANDS:
    deb       pack into debian package
    macos     pack into macOS .pkg
    tarballs  packages into tarballs
    win       create windows installer
"
COMMAND="$1"

if [ -z "$COMMAND" ]
then
  echo "$cmd_usage"
  exit 1
fi

FLAGS=""

# Node 24+ does not publish 32-bit binaries (linux-armv7l, win-x86), so we
# must drop those targets from oclif's defaults; otherwise `oclif pack` 404s
# while downloading the embedded Node runtime.
#   - `tarballs` and `win` accept `--targets`, so we override on the command line.
#   - `oclif pack deb` has no `--targets` flag; its linux targets come from
#     `oclif.update.node.targets` in packages/dashmate/package.json.
case "$COMMAND" in
  tarballs)
    FLAGS="--no-xz --targets=linux-arm64,linux-x64"
    ;;
  win)
    FLAGS="--targets=win32-x64"
    ;;
esac

FULL_PATH=$(realpath "$0")
DIR_PATH=$(dirname "$FULL_PATH")
ROOT_PATH=$(dirname "$DIR_PATH")

# oclif hardcodes the deb version as `<major.minor.patch>.<git sha>-1`. That drops the
# semver prerelease tag and makes the git sha an ordering component: dpkg orders digits
# as equal and letters by character code, so a sha starting with a digit sorts below one
# starting with a letter. Apt then reads about half of the releases as downgrades, and a
# rebuild of an already published version is never offered at all.
#
# Rebuild the packages with the Debian idiom instead (4.1.0-1, 4.1.0~rc.3-1, rebuilds
# bumping the Debian revision through DASHMATE_DEB_REVISION; see deb-version.mjs) and carry
# the git sha in the package description, where it has no ordering weight. The packages go
# back through dpkg-deb rather than being patched in place so they keep whatever archive
# format and compression the system dpkg produces. Nothing else in the control file has to
# be kept in step with the payload, because oclif's template declares neither md5sums nor
# Installed-Size.
#
# The file name is not the version: it carries no epoch, and no `~`, so that the name
# survives being uploaded as a release asset.
rewrite_deb_versions() {
  DIST_PATH="$1"
  STAGING_PATH="tmp/rewritten"

  if ! ls "$DIST_PATH"/*.deb > /dev/null 2>&1
  then
    echo "No deb packages to rewrite in $DIST_PATH"
    exit 1
  fi

  SEMVER_VERSION=$(node -p "require('./package.json').version")
  DEB_VERSION=$(node "$ROOT_PATH/packages/dashmate/scripts/deb-version.mjs" "$SEMVER_VERSION")
  DEB_FILE_VERSION=$(node "$ROOT_PATH/packages/dashmate/scripts/deb-version.mjs" --file-name "$SEMVER_VERSION")

  rm -rf "$STAGING_PATH"
  mkdir -p "$STAGING_PATH"

  # Every package is rebuilt into a staging directory and only swapped in once they have
  # all succeeded, so a failure part way through cannot leave a mix of old and new
  # packages behind.
  for DEB_PATH in "$DIST_PATH"/*.deb
  do
    PACKAGE=$(dpkg-deb --field "$DEB_PATH" Package)
    ARCH=$(dpkg-deb --field "$DEB_PATH" Architecture)
    OCLIF_VERSION=$(dpkg-deb --field "$DEB_PATH" Version)

    GIT_SHA=${OCLIF_VERSION##*.}
    GIT_SHA=${GIT_SHA%%-*}

    if ! echo "$GIT_SHA" | grep -Eq '^[0-9a-f]{7,40}$'
    then
      echo "Expected a git sha in the version $OCLIF_VERSION built by oclif"
      exit 1
    fi

    WORKSPACE_PATH="tmp/repack/$ARCH"
    rm -rf "$WORKSPACE_PATH"
    # dpkg-deb creates the last path component only.
    mkdir -p "$(dirname "$WORKSPACE_PATH")"
    dpkg-deb --raw-extract "$DEB_PATH" "$WORKSPACE_PATH"

    # The sha is appended to the end of the description, after any lines continuing it.
    # It is the only record of which commit the package was built from, since the version
    # does not carry it, so a control file without a description to attach it to is an
    # error rather than something to skip quietly. The description is closed before any
    # other field is matched, so the sha cannot end up under a field that follows it.
    if ! awk -v version="$DEB_VERSION" -v sha="$GIT_SHA" '
      in_description && /^[ \t]/ { print; next }
      in_description { print " Built from git commit " sha "."; in_description = 0 }
      /^Version: / { print "Version: " version; next }
      /^Description: / { found = 1; in_description = 1; print; next }
      { print }
      END {
        if (in_description) print " Built from git commit " sha "."
        if (!found) exit 1
      }
    ' "$WORKSPACE_PATH/DEBIAN/control" > "$WORKSPACE_PATH/DEBIAN/control.rewritten"
    then
      echo "No Description field in the control file of $DEB_PATH to record the git sha in"
      exit 1
    fi

    mv "$WORKSPACE_PATH/DEBIAN/control.rewritten" "$WORKSPACE_PATH/DEBIAN/control"

    # oclif builds the payload as root, so restore that after extracting and rebuilding
    # it as the current user.
    REWRITTEN_PATH="$STAGING_PATH/${PACKAGE}_${DEB_FILE_VERSION}_${ARCH}.deb"
    dpkg-deb --root-owner-group --build "$WORKSPACE_PATH" "$REWRITTEN_PATH"

    # A control file whose Version line the rewrite does not recognise would otherwise ship
    # oclif's version under the new file name.
    if [ "$(dpkg-deb --field "$REWRITTEN_PATH" Version)" != "$DEB_VERSION" ]
    then
      echo "Could not rewrite the Version field of $DEB_PATH to $DEB_VERSION"
      exit 1
    fi

    rm -rf "$WORKSPACE_PATH"
  done

  rm -f "$DIST_PATH"/*.deb
  mv "$STAGING_PATH"/*.deb "$DIST_PATH"
  rm -rf "$STAGING_PATH"

  echo "Rewrote deb packages as $DEB_VERSION, published as ${DEB_FILE_VERSION}"
}

cd $ROOT_PATH/packages/dashmate || exit 1
yarn pack --install-if-needed
tar zxvf package.tgz -C .
cd $ROOT_PATH/packages/dashmate/package || exit 1
cp $ROOT_PATH/yarn.lock ./yarn.lock
mkdir .yarn
echo "nodeLinker: node-modules"  > .yarnrc.yml
# CI signs the finished installer in a separate job with access to the Apple
# keychain. Keep the normal local signing behavior unless explicitly disabled.
if [ "$COMMAND" = macos ] && [ "${DASHMATE_UNSIGNED:-false}" = true ]; then
  node -e 'const fs = require("fs"); const p = JSON.parse(fs.readFileSync("package.json", "utf8")); delete p.oclif.macos.sign; fs.writeFileSync("package.json", JSON.stringify(p, null, 2) + "\n");'
fi
# The release workflow passes the npm tarballs it publishes. Install the
# workspace packages from them rather than from the registry, which can take
# many minutes to serve a version it has just accepted. The paths must be
# absolute: `oclif pack` installs again from a copy of this lockfile in its
# own tmp directory.
if [ -n "${DASHMATE_NPM_TARBALLS_DIR:-}" ]; then
  DASHMATE_NPM_TARBALLS_DIR="$(realpath "$DASHMATE_NPM_TARBALLS_DIR")" node -e '
    const fs = require("fs");
    const path = require("path");
    const { execFileSync } = require("child_process");
    const dir = process.env.DASHMATE_NPM_TARBALLS_DIR;
    const p = JSON.parse(fs.readFileSync("package.json", "utf8"));
    const tarballs = fs.readdirSync(dir).filter((file) => file.endsWith(".tgz"));
    if (tarballs.length === 0) throw new Error(`No npm tarballs in ${dir}`);
    // Keep the entries the package ships with: one of them leaves out
    // cpu-features, whose native build Apple does not notarize.
    p.resolutions = p.resolutions || {};
    for (const file of tarballs) {
      const tarball = path.join(dir, file);
      const { name } = JSON.parse(execFileSync("tar", ["-xOzf", tarball, "package/package.json"], { encoding: "utf8" }));
      if (name !== p.name) p.resolutions[name] = `file:${tarball}`;
    }
    fs.writeFileSync("package.json", JSON.stringify(p, null, 2) + "\n");
  '
fi
yarn install --no-immutable
yarn oclif manifest
yarn oclif pack $COMMAND $FLAGS

if [ "$COMMAND" = "deb" ]
then
  rewrite_deb_versions "dist/deb"
fi

cd ..  || exit 1
rm package.tgz
# oclif writes an apt index (Packages*, Release) next to the debs. Releases publish the
# packages only, and the index would also still list the file names and versions that
# rewrite_deb_versions replaced.
rm -f package/dist/deb/Packages* package/dist/deb/Release package/dist/deb/InRelease package/dist/deb/Release.gpg
cp -R package/dist "$ROOT_PATH/packages/dashmate"

# fix for deb package build
sudo chown -R $USER "$ROOT_PATH/packages/dashmate/package" || true
sudo chgrp -R $USER "$ROOT_PATH/packages/dashmate/package" || true

# remove build folder
rm -rf package

echo "Done"
