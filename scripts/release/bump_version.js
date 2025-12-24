const fs = require('fs');
const path = require('path');
const semver = require('semver');
const TOML = require('@iarna/toml');
const packagesIterator = require('../utils/packagesIterator');
const rootPackageJson = require('../../package.json');
const rootCargoTomlPath = path.join(__dirname, '..', '..', 'Cargo.toml');

const convertReleaseToPrerelease = (version, prereleaseType) => {
  const bumpedVersion = semver.inc(version, 'minor');

  return `${semver.major(bumpedVersion)}.${semver.minor(bumpedVersion)}.0-${prereleaseType}.1`;
};

const convertPrereleaseType = (version, prereleaseType) => {
  return `${semver.major(version)}.${semver.minor(version)}.0-${prereleaseType}.1`;
};

const setExactVersion = (targetVersion) => () => targetVersion;

const parseArgs = (argv) => {
  let releaseType;
  let targetVersion;

  for (let i = 0; i < argv.length; i += 1) {
    const arg = argv[i];

    if (arg.startsWith('--target-version=')) {
      targetVersion = arg.split('=')[1];
      continue;
    }

    if (arg === '--target-version') {
      targetVersion = argv[i + 1];
      i += 1;
      continue;
    }

    if (arg.startsWith('--version=')) {
      targetVersion = arg.split('=')[1];
      continue;
    }

    if (arg === '--version') {
      targetVersion = argv[i + 1];
      i += 1;
      continue;
    }

    if (!releaseType && !arg.startsWith('-')) {
      releaseType = arg;
    }
  }

  return { releaseType, targetVersion };
};

const bumpNpmPackages = (versionFunc, releaseType) => {
  for (const { filename, json } of packagesIterator.npm()) {
    const { version } = json;

    json.version = versionFunc(version, releaseType);

    fs.writeFileSync(filename, `${JSON.stringify(json, null, 2)}\n`);
  }
}

const bumpRustPackages = (versionFunc, releaseType) => {
  const cargoFile = fs.readFileSync(rootCargoTomlPath, 'utf-8');
  const parsedCargo = TOML.parse(cargoFile);

  const currentVersion = parsedCargo?.workspace?.package?.version;

  if (!currentVersion) {
    throw new Error('Unable to determine workspace package version from Cargo.toml');
  }

  const nextVersion = versionFunc(currentVersion, releaseType);

  const replaceFrom = `version = "${currentVersion}"`;
  const replaceTo = `version = "${nextVersion}"`;

  fs.writeFileSync(rootCargoTomlPath, cargoFile.replace(replaceFrom, replaceTo));
}

(async () => {
  const { releaseType: releaseTypeArg, targetVersion } = parseArgs(process.argv.slice(2));

  const { version: rootVersion } = rootPackageJson;

  let rootVersionType = 'release';

  const semverPrerelease = semver.prerelease(rootVersion);
  if (semverPrerelease !== null) {
    rootVersionType = semverPrerelease[0];
  }

  let releaseType = releaseTypeArg;

  if (targetVersion !== undefined && !semver.valid(targetVersion)) {
    throw new Error(`Invalid target version: ${targetVersion}`);
  }

  if (targetVersion !== undefined && releaseType === undefined) {
    const targetPrerelease = semver.prerelease(targetVersion);
    releaseType = targetPrerelease !== null ? targetPrerelease[0] : 'release';
  }

  // Figure out release type using current version if not set
  if (releaseType === undefined) {
    // get releaseType from root package.json
    releaseType = rootVersionType;
  }

  if (targetVersion !== undefined) {
    const targetPrerelease = semver.prerelease(targetVersion);
    const targetVersionType = targetPrerelease !== null ? targetPrerelease[0] : 'release';

    if (releaseType !== targetVersionType) {
      throw new Error(`Specified release type (${releaseType}) does not match target version type (${targetVersionType})`);
    }

    bumpNpmPackages(setExactVersion(targetVersion), releaseType);
    bumpRustPackages(setExactVersion(targetVersion), releaseType);

    rootPackageJson.version = targetVersion;
    fs.writeFileSync(path.join(__dirname, '..', '..', 'package.json'), `${JSON.stringify(rootPackageJson, null, 2)}\n`);

    return;
  }

  if (rootVersionType === releaseType && releaseType === 'release') {
    // release to release
    bumpNpmPackages(semver.inc, 'patch');
    bumpRustPackages(semver.inc, 'patch');

    // root version
    rootPackageJson.version = semver.inc(rootPackageJson.version, 'patch');
    fs.writeFileSync(path.join(__dirname, '..', '..', 'package.json'), `${JSON.stringify(rootPackageJson, null, 2)}\n`);
  } else if (rootVersionType === 'release' && releaseType !== 'release') {
    // release to prerelease
    bumpNpmPackages(convertReleaseToPrerelease, releaseType);
    bumpRustPackages(convertReleaseToPrerelease, releaseType);

    // root version
    rootPackageJson.version = convertReleaseToPrerelease(rootPackageJson.version, releaseType);
    fs.writeFileSync(path.join(__dirname, '..', '..', 'package.json'), `${JSON.stringify(rootPackageJson, null, 2)}\n`);
  } else if (rootVersionType !== 'release' && releaseType === 'release') {
    // prerelease to release
    bumpNpmPackages(semver.inc, 'minor');
    bumpRustPackages(semver.inc, 'minor');

    // root version
    rootPackageJson.version = semver.inc(rootPackageJson.version, 'minor');
    fs.writeFileSync(path.join(__dirname, '..', '..', 'package.json'), `${JSON.stringify(rootPackageJson, null, 2)}\n`);
  } else if (rootVersionType !== releaseType) {
    // dev to alpha or vice versa
    bumpNpmPackages(convertPrereleaseType, releaseType);
    bumpRustPackages(convertPrereleaseType, releaseType);

    // root version
    rootPackageJson.version = convertPrereleaseType(rootPackageJson.version, releaseType);
    fs.writeFileSync(path.join(__dirname, '..', '..', 'package.json'), `${JSON.stringify(rootPackageJson, null, 2)}\n`);
  } else {
    // prerelease to prerelease (the same type)
    bumpNpmPackages(semver.inc, 'prerelease');
    bumpRustPackages(semver.inc, 'prerelease');

    // root version
    rootPackageJson.version = semver.inc(rootPackageJson.version, 'prerelease');
    fs.writeFileSync(path.join(__dirname, '..', '..', 'package.json'), `${JSON.stringify(rootPackageJson, null, 2)}\n`);
  }
})();
