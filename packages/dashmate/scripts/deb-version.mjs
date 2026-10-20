/**
 * Translate a semver version into a Debian package version, and into the file name that
 * version is published under.
 *
 * oclif builds the deb version as `<major.minor.patch>.<git sha>-1`. That drops the
 * semver prerelease tag and turns the git sha into an ordering component: dpkg orders
 * digits as equal and letters by their character code, so a sha starting with a digit
 * sorts below one starting with a letter. Roughly half of the releases published that
 * way look like downgrades to apt, and a rebuild of an already published version can
 * never be installed at all.
 *
 * Debian's own idiom is used instead. `~` sorts below everything, including the end of
 * the string, so a prerelease stays below its final release, and rebuilds of the same
 * upstream version are distinguished by the Debian revision:
 *
 *   4.1.0                    -> 4.1.0-1
 *   4.1.0-rc.3               -> 4.1.0~rc.3-1
 *   4.1.0-dev.11             -> 4.1.0~~dev.11-1
 *   4.1.0, second build      -> 4.1.0-2
 *
 * Releases go through dev, then beta, then rc. Prerelease labels compare by their
 * characters, so `~dev` would sort above `~beta` and the first beta would look like a
 * downgrade. A second `~` sorts below a single one, which keeps dev builds below every
 * other prerelease of the same version.
 *
 * The git sha is not part of the version; it is carried in the package description.
 *
 * An epoch is available for the one case the scheme cannot express: an upstream version
 * that was already published under the git sha scheme, where `4.1.0-1` sorts below the
 * published `4.1.0.bfc80249b9-1` because the sha extends the upstream version.
 */

import { fileURLToPath } from 'node:url';

// Version identifiers follow semver: no leading zeros, because `4.1.0-rc.01` and
// `4.1.0-rc.1` are two distinct tags that dpkg considers the same version.
const NUMERIC_IDENTIFIER = '(?:0|[1-9]\\d*)';
// The prerelease is deliberately restricted to alphanumerics and dots. A `-` there would
// end up in the Debian upstream version and move where dpkg splits off the revision.
const PRERELEASE_IDENTIFIER = '(?:0|[1-9]\\d*|\\d*[A-Za-z][0-9A-Za-z]*)';

// Build metadata (`+...`) is not accepted: no release is tagged with it, and semver gives it
// no weight in ordering, so it has no place in a version apt orders by.
const VERSION_REGEX = new RegExp(`^v?(${NUMERIC_IDENTIFIER}\\.${NUMERIC_IDENTIFIER}\\.${NUMERIC_IDENTIFIER})`
  + `(?:-(${PRERELEASE_IDENTIFIER}(?:\\.${PRERELEASE_IDENTIFIER})*))?$`);

// Both are set by hand when a release needs them, and dpkg reads `01` as `1`, so a leading
// zero would produce a version that is no higher than the one it was meant to overtake.
const REVISION_REGEX = /^[1-9]\d*$/;
const EPOCH_REGEX = /^[1-9]\d*$/;

const DEV_PRERELEASE = 'dev';

/**
 * @param {string} version - semver version, with or without a leading `v`
 * @param {object} [options]
 * @param {string} [options.revision] - Debian revision, bumped for rebuilds of one version
 * @param {string} [options.epoch] - Debian epoch, omitted when empty
 * @returns {string}
 */
function debVersionFromSemver(version, options = {}) {
  const { revision = '1', epoch = '' } = options;

  const parsed = VERSION_REGEX.exec(String(version).trim());

  if (parsed === null) {
    throw new Error(`"${version}" is not a version that can be translated to a Debian version.`
      + ' Expected MAJOR.MINOR.PATCH with an optional alphanumeric prerelease, for example'
      + ' 4.1.0 or 4.1.0-rc.3');
  }

  if (!REVISION_REGEX.test(String(revision))) {
    throw new Error(`"${revision}" is not a valid Debian revision. Expected a positive number`
      + ' without leading zeros');
  }

  if (epoch !== '' && !EPOCH_REGEX.test(String(epoch))) {
    throw new Error(`"${epoch}" is not a valid Debian epoch. Expected a positive number without`
      + ' leading zeros');
  }

  const [, release, prerelease] = parsed;

  let upstream = release;

  if (prerelease !== undefined) {
    upstream += prerelease.split('.')[0] === DEV_PRERELEASE ? `~~${prerelease}` : `~${prerelease}`;
  }

  return `${epoch === '' ? '' : `${epoch}:`}${upstream}-${revision}`;
}

/**
 * The version as it appears in the package file name.
 *
 * Debian leaves the epoch out of file names. Each run of `~` becomes a single `.`, because
 * GitHub rewrites `~` when a release asset is uploaded, and the name has to be known before
 * the upload. Only the name changes; the version apt installs comes from the control file.
 *
 * @param {string} debVersion - as returned by debVersionFromSemver
 * @returns {string}
 */
function debFileNameVersion(debVersion) {
  return debVersion.replace(/^\d+:/, '').replace(/~+/g, '.');
}

export { debVersionFromSemver, debFileNameVersion };

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  const args = process.argv.slice(2);
  const forFileName = args[0] === '--file-name';
  const version = forFileName ? args[1] : args[0];
  const expectedArgumentCount = forFileName ? 2 : 1;

  if (!version || args.length !== expectedArgumentCount) {
    // eslint-disable-next-line no-console
    console.error('Usage: deb-version.mjs [--file-name] SEMVER_VERSION\n\n'
      + '  Prints the Debian package version for a semver version, or the version as it\n'
      + '  appears in the package file name.\n\n'
      + '  DASHMATE_DEB_REVISION  Debian revision, default 1. Bump it to rebuild a version\n'
      + '                         that was already published.\n'
      + '  DASHMATE_DEB_EPOCH     Debian epoch, unset by default.\n');

    process.exit(1);
  }

  try {
    const debVersion = debVersionFromSemver(version, {
      revision: process.env.DASHMATE_DEB_REVISION || '1',
      epoch: process.env.DASHMATE_DEB_EPOCH || '',
    });

    // eslint-disable-next-line no-console
    console.log(forFileName ? debFileNameVersion(debVersion) : debVersion);
  } catch (e) {
    // eslint-disable-next-line no-console
    console.error(e.message);

    process.exit(1);
  }
}
