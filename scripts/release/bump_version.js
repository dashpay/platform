const fs = require('fs');
const path = require('path');
const semver = require('semver');
const packagesIterator = require('../utils/packagesIterator');
const rootPackageJson = require('../../package.json');

const prereleaseTypes = new Set(['dev', 'alpha']);

const convertReleaseToPrerelease = (version, prereleaseType = 'dev') => {
  const bumpedVersion = semver.inc(version, 'minor');

  return `${semver.major(bumpedVersion)}.${semver.minor(bumpedVersion)}.0-${prereleaseType}.1`;
};

(async () => {
  let [ releaseType ] = process.argv.slice(2);

  const packagesDir = path.join(__dirname, '..', '..', 'packages');
  const { version: rootVersion } = rootPackageJson;

  let rootVersionType = 'release';

  const semverPrerelease = semver.prerelease(rootVersion);
  if (semverPrerelease !== null) {
    rootVersionType = semverPrerelease[0];
  }

  // Figure out release type using current version if not set
  if (releaseType === undefined) {
    // get releaseType from root package.json
    releaseType = rootVersionType;
  }

  if (rootVersionType === releaseType && releaseType === 'release') {
    // release to release
    for (const { filename, json } of packagesIterator(packagesDir)) {
      const { version } = json;
      json.version = semver.inc(version, 'patch');

      fs.writeFileSync(filename, `${JSON.stringify(json, null, 2)}\n`);
    }

    // root version
    rootPackageJson.version = semver.inc(rootPackageJson.version, 'patch');
    fs.writeFileSync(path.join(__dirname, '..', '..', 'package.json'), `${JSON.stringify(rootPackageJson, null, 2)}\n`);
  } else if (rootVersionType === 'release' && prereleaseTypes.has(releaseType)) {
    // release to prerelease
    for (const { filename, json } of packagesIterator(packagesDir)) {
      const { version } = json;
      json.version = convertReleaseToPrerelease(version, releaseType);

      fs.writeFileSync(filename, `${JSON.stringify(json, null, 2)}\n`);
    }

    // root version
    rootPackageJson.version = convertReleaseToPrerelease(rootPackageJson.version);
    fs.writeFileSync(path.join(__dirname, '..', '..', 'package.json'), `${JSON.stringify(rootPackageJson, null, 2)}\n`);
  } else if (prereleaseTypes.has(rootVersionType) && prereleaseTypes.has(releaseType)) {
    // prerelease to release
    for (const { filename, json } of packagesIterator(packagesDir)) {
      const { version } = json;
      json.version = semver.inc(version, 'minor');

      fs.writeFileSync(filename, `${JSON.stringify(json, null, 2)}\n`);
    }

    // root version
    rootPackageJson.version = semver.inc(rootPackageJson.version, 'minor');
    fs.writeFileSync(path.join(__dirname, '..', '..', 'package.json'), `${JSON.stringify(rootPackageJson, null, 2)}\n`);
  } else {
    // prerelease to prerelease
    for (const { filename, json } of packagesIterator(packagesDir)) {
      const { version } = json;
      json.version = semver.inc(version, 'prerelease');

      fs.writeFileSync(filename, `${JSON.stringify(json, null, 2)}\n`);
    }

    // root version
    rootPackageJson.version = semver.inc(rootPackageJson.version, 'prerelease');
    fs.writeFileSync(path.join(__dirname, '..', '..', 'package.json'), `${JSON.stringify(rootPackageJson, null, 2)}\n`);
  }
})();
