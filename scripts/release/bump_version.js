const fs = require('fs');
const path = require('path');
const semver = require('semver');
const packagesIterator = require('../utils/packagesIterator');
const rootPackageJson = require('../../package.json');

const convertReleaseToPrerelease = (version, prereleaseType) => {
  const bumpedVersion = semver.inc(version, 'minor');

  return `${semver.major(bumpedVersion)}.${semver.minor(bumpedVersion)}.0-${prereleaseType}.1`;
};

const convertPrereleaseType = (version, prereleaseType) => {
  return `${semver.major(version)}.${semver.minor(version)}.0-${prereleaseType}.1`;
};

const handlePackages = (versionFunc, releaseType) => {
  for (const { filename, json, toml } of packagesIterator())  {
    if (json) {
      const { version } = json;
      
      json.version = versionFunc(version, releaseType);

      fs.writeFileSync(filename, `${JSON.stringify(json, null, 2)}\n`);
    }

    if (toml) {
      const {version} = toml.package

      const tomlVersion = versionFunc(version, releaseType);

      const cargoFile = fs.readFileSync(filename, 'utf-8')

      const replaceFrom = `version = "${version}"`
      const replaceTo = `version = "${tomlVersion}"`

      fs.writeFileSync(filename, cargoFile.replace(replaceFrom, replaceTo));
    }
  }
}

(async () => {
  let [ releaseType ] = process.argv.slice(2);

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
    handlePackages(semver.inc, 'patch')

    // root version
    rootPackageJson.version = semver.inc(rootPackageJson.version, 'patch');
    fs.writeFileSync(path.join(__dirname, '..', '..', 'package.json'), `${JSON.stringify(rootPackageJson, null, 2)}\n`);
  } else if (rootVersionType === 'release' && releaseType !== 'release') {
    // release to prerelease
    handlePackages(convertReleaseToPrerelease, releaseType)

    // root version
    rootPackageJson.version = convertReleaseToPrerelease(rootPackageJson.version, releaseType);
    fs.writeFileSync(path.join(__dirname, '..', '..', 'package.json'), `${JSON.stringify(rootPackageJson, null, 2)}\n`);
  } else if (rootVersionType !== 'release' && releaseType === 'release') {
    // prerelease to release
    handlePackages(semver.inc, 'minor')

    // root version
    rootPackageJson.version = semver.inc(rootPackageJson.version, 'minor');
    fs.writeFileSync(path.join(__dirname, '..', '..', 'package.json'), `${JSON.stringify(rootPackageJson, null, 2)}\n`);
  } else if (rootVersionType !== releaseType) {
    // dev to alpha or vice versa
    handlePackages(convertPrereleaseType, releaseType)

    // root version
    rootPackageJson.version = convertPrereleaseType(rootPackageJson.version, releaseType);
    fs.writeFileSync(path.join(__dirname, '..', '..', 'package.json'), `${JSON.stringify(rootPackageJson, null, 2)}\n`);
  } else {
    // prerelease to prerelease (the same type)
    handlePackages(semver.inc, 'prerelease')

    // root version
    rootPackageJson.version = semver.inc(rootPackageJson.version, 'prerelease');
    fs.writeFileSync(path.join(__dirname, '..', '..', 'package.json'), `${JSON.stringify(rootPackageJson, null, 2)}\n`);
  }
})();
