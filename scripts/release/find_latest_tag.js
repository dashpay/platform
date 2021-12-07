const semver = require('semver');
const execute = require('../utils/execute');

const [ version ] = process.argv.slice(2);

if (!version) {
  console.log('usage example: yarn node find_latest_tag.js v0.21.0');
  process.exit(1);
}

(async () => {
  const tags = (await execute('git tag -l --sort=-v:refname'));

  const isPrerelease = semver.prerelease(version) !== null;
  const parsedVersion = semver.parse(version);

  let result;

  if (!isPrerelease) {
    // stable

    // try to find the latest stable version with same minor part
    result = tags.match(new RegExp(`^v${parsedVersion.major}\.${parsedVersion.minor}\.([0-9]+)$`, 'mgi'));

    // try to find the latest stable version with previous minor part
    if (!result) {
      result = tags.match(new RegExp(`^v${parsedVersion.major}\.${parsedVersion.minor - 1}\.([0-9]+)$`, 'mgi'));
    }
  } else {
    // prerelease

    // try to find previous prerelease
    result = tags.match(new RegExp(`^v${parsedVersion.major}\.${parsedVersion.minor}\.0-dev.([0-9]+)$`, 'mgi'));

    if (!result) {
      // try to find the latest stable version with previous minor part
      result = tags.match(new RegExp(`^v${parsedVersion.major}\.${parsedVersion.minor - 1}\.([0-9]+)$`, 'mgi'));
    }
  }

  if (!result) {
    console.log(`Can't find latest tag for the version ${version}`);
    process.exit(1);
  }

  console.log(result[0]);
})();
