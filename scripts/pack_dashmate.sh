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
# bumping the Debian revision through DASHMATE_DEB_REVISION) and carry the git sha in the
# package description, where it has no ordering weight. The packages go back through
# dpkg-deb rather than being patched in place so they keep whatever archive format and
# compression the system dpkg produces. Nothing else in the control file has to be kept in
# step with the payload, because oclif's template declares neither md5sums nor
# Installed-Size.
#
# The file name is not the version: it carries no epoch, and no `~`, so that the name
# survives being uploaded as a release asset and still matches the index that points at it.
rewrite_deb_versions() {
  DIST_PATH="$1"
  STAGING_PATH="tmp/rewritten"

  if ! ls "$DIST_PATH"/*.deb > /dev/null 2>&1
  then
    echo "No deb packages to rewrite in $DIST_PATH"
    exit 1
  fi

  SEMVER_VERSION=$(node -p "require('./package.json').version")
  DEB_VERSION=$(node "$DIR_PATH/deb_version.js" "$SEMVER_VERSION")
  DEB_FILE_VERSION=$(node "$DIR_PATH/deb_version.js" --file-name "$SEMVER_VERSION")

  rm -rf "$STAGING_PATH"
  mkdir -p "$STAGING_PATH"

  # Every package is rebuilt into a staging directory and only swapped in once they have
  # all succeeded, so a failure part way through cannot leave the indexes describing a
  # mix of old and new file names.
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
    # It is the only record of which commit the package was built from now that it is out
    # of the version, so a control file without a description to attach it to is an error
    # rather than something to skip quietly.
    if ! awk -v version="$DEB_VERSION" -v sha="$GIT_SHA" '
      /^Version: / { print "Version: " version; next }
      /^Description: / { found = 1; in_description = 1; print; next }
      in_description && /^[ \t]/ { print; next }
      in_description { print " Built from git commit " sha "."; in_description = 0 }
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
    dpkg-deb --root-owner-group --build "$WORKSPACE_PATH" "$STAGING_PATH/${PACKAGE}_${DEB_FILE_VERSION}_${ARCH}.deb"

    rm -rf "$WORKSPACE_PATH"
  done

  rm -f "$DIST_PATH"/*.deb
  mv "$STAGING_PATH"/*.deb "$DIST_PATH"
  rm -rf "$STAGING_PATH"

  # Apt takes the version and the file name from these indexes, so they have to be built
  # again around the renamed packages.
  ORIGIN=$(sed -n 's/^Origin: //p' "$DIST_PATH/Release")
  SUITE=$(sed -n 's/^Suite: //p' "$DIST_PATH/Release")

  if [ -z "$ORIGIN" ] || [ -z "$SUITE" ]
  then
    echo "Could not read Origin and Suite from $DIST_PATH/Release"
    exit 1
  fi

  # Kept beside the build rather than in a temporary directory so it goes away with the
  # rest of the build even if the script stops early, and out of the indexed directory so
  # apt-ftparchive does not list it.
  FTPARCHIVE_CONF="$PWD/tmp/apt-ftparchive.conf"

  printf 'APT::FTPArchive::Release {\n  Origin "%s";\n  Suite "%s";\n};\n' "$ORIGIN" "$SUITE" > "$FTPARCHIVE_CONF"

  (
    cd "$DIST_PATH" || exit 1
    rm -f Packages Packages.gz Packages.bz2 Packages.xz Release InRelease Release.gpg
    apt-ftparchive packages . > Packages
    gzip -c Packages > Packages.gz
    bzip2 -c Packages > Packages.bz2
    xz -c Packages > Packages.xz
    apt-ftparchive -c "$FTPARCHIVE_CONF" release . > Release

    # The signatures oclif made cover the metadata from before the rewrite.
    if [ -n "$DASHMATE_DEB_KEY" ]
    then
      gpg --digest-algo SHA512 --clearsign -u "$DASHMATE_DEB_KEY" -o InRelease Release
      gpg --digest-algo SHA512 -abs -u "$DASHMATE_DEB_KEY" -o Release.gpg Release
    fi
  )

  rm -f "$FTPARCHIVE_CONF"

  echo "Rewrote deb packages as $DEB_VERSION, published as ${DEB_FILE_VERSION}"
}

cd $ROOT_PATH/packages/dashmate || exit 1
yarn pack --install-if-needed
tar zxvf package.tgz -C .
cd $ROOT_PATH/packages/dashmate/package || exit 1
cp $ROOT_PATH/yarn.lock ./yarn.lock
mkdir .yarn
echo "nodeLinker: node-modules"  > .yarnrc.yml
yarn install --no-immutable
yarn oclif manifest
yarn oclif pack $COMMAND $FLAGS

if [ "$COMMAND" = "deb" ]
then
  rewrite_deb_versions "dist/deb"
fi

cd ..  || exit 1
rm package.tgz
cp -R package/dist "$ROOT_PATH/packages/dashmate"

# fix for deb package build
sudo chown -R $USER "$ROOT_PATH/packages/dashmate/package" || true
sudo chgrp -R $USER "$ROOT_PATH/packages/dashmate/package" || true

# remove build folder
rm -rf package

echo "Done"
