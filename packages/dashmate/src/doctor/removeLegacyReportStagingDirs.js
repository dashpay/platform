import fs from 'fs';
import path from 'path';
import process from 'process';

const LEGACY_STAGING_DIR_PREFIX = 'dashmate-report-';

/**
 * The name earlier versions staged a report under: the report name with the
 * collection time, and nothing after it. The current version adds a random
 * suffix, so a report being written right now never matches.
 */
const LEGACY_STAGING_DIR_NAME = /^dashmate-report-\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\.\d{3}Z$/;

/**
 * The files earlier versions wrote at the top of a staging directory, and in
 * the directory of each service below it: every sample they collected, under
 * its name, as text or as JSON. Nothing else was ever written there, so
 * anything else - notes beside an unpacked report, say - is not theirs to
 * remove.
 */
const LEGACY_ROOT_SAMPLES = [
  'activation', 'dashmateConfig', 'dashmateVersion', 'date', 'dockerError', 'systemInfo',
];

/**
 * Every service earlier versions could run, each with the samples of its
 * container and those collected for it alone
 */
const LEGACY_CONTAINER_SAMPLES = ['dockerInspect', 'dockerStats', 'stdErr', 'stdOut'];

const LEGACY_SERVICE_SAMPLES = {
  core: ['bestChainLock', 'blockchainInfo', 'masternodeStatus', 'masternodeSyncStatus', 'p2pPort', 'peerInfo', 'quorums'],
  drive_abci: ['metrics'],
  drive_tenderdash: ['abciInfo', 'consensusState', 'genesis', 'metrics', 'nodeIdFromKey', 'p2pPort', 'peers', 'status', 'validators'],
  gateway: [
    'acmeHttpPort', 'certificateRenewal', 'httpPort', 'installedCertificate', 'metrics',
    'servedCertificate', 'ssl', 'sslPrivateKeyMode', 'validationHttpPort',
  ],
};

const LEGACY_SERVICES = [
  'core', 'core_insight', 'core_tor', 'dapi_api', 'dapi_core_streams', 'dapi_envoy',
  'dapi_envoy_rate_limiter', 'dapi_envoy_rate_limiter_metrics', 'dapi_envoy_rate_limiter_redis',
  'dapi_insight', 'dapi_nginx', 'dapi_tx_filter_stream', 'dashmate_helper', 'drive_abci',
  'drive_tenderdash', 'gateway', 'gateway_rate_limiter', 'gateway_rate_limiter_metrics',
  'gateway_rate_limiter_redis', 'quorum_list', 'rs_dapi',
];

/**
 * @param {string} service
 * @return {string[]}
 */
function getLegacyServiceSamples(service) {
  return [...LEGACY_CONTAINER_SAMPLES, ...(LEGACY_SERVICE_SAMPLES[service] ?? [])];
}

/**
 * @param {string} fileName
 * @param {string[]} samples
 * @return {boolean}
 */
function isSampleFileName(fileName, samples) {
  const extension = path.extname(fileName);

  return ['.txt', '.json'].includes(extension)
    && samples.includes(path.basename(fileName, extension));
}

/**
 * @param {string} name
 * @return {boolean}
 */
function isLegacyStagingDirName(name) {
  if (!LEGACY_STAGING_DIR_NAME.test(name)) {
    return false;
  }

  // Only a time earlier versions could have written: the shape alone also
  // fits dates that do not exist
  const time = new Date(name.slice(LEGACY_STAGING_DIR_PREFIX.length));

  return !Number.isNaN(time.getTime())
    && time.toISOString() === name.slice(LEGACY_STAGING_DIR_PREFIX.length);
}

/**
 * Whether only its owner, or root, can rename or remove what a directory holds
 *
 * @param {fs.Stats} stat
 * @param {number} uid
 * @return {boolean}
 */
function isGuardedDirectory(stat, uid) {
  if (stat.uid !== 0 && stat.uid !== uid) {
    return false;
  }

  // eslint-disable-next-line no-bitwise
  const writableByOthers = (stat.mode & 0o022) !== 0;
  // eslint-disable-next-line no-bitwise
  const sticky = (stat.mode & 0o1000) !== 0;

  // A shared temp directory is writable by everyone, but sticky: an entry
  // there can only be renamed or removed by whoever owns it
  return !writableByOthers || sticky;
}

/**
 * @param {string} dir - a resolved path
 * @param {number} uid
 * @return {boolean} whether nobody else can swap the directory, or any above
 *   it, for another one
 */
function isGuardedPath(dir, uid) {
  for (let current = dir; ; current = path.dirname(current)) {
    if (!isGuardedDirectory(fs.lstatSync(current), uid)) {
      return false;
    }

    if (path.dirname(current) === current) {
      return true;
    }
  }
}

/**
 * Open a directory without following links and, through that handle, check it
 * is the user's own on the expected file system and close it to everyone
 * else. A link put in its place is never followed, and nothing in it can be
 * renamed or replaced by anyone else afterwards.
 *
 * @param {string} dir
 * @param {number} uid
 * @param {number} dev
 */
function closeDirectory(dir, uid, dev) {
  // eslint-disable-next-line no-bitwise
  const flags = fs.constants.O_RDONLY | fs.constants.O_NOFOLLOW | fs.constants.O_DIRECTORY;

  let fd;

  try {
    fd = fs.openSync(dir, flags);
  } catch (error) {
    if (error.code !== 'EACCES') {
      throw error;
    }

    // A directory its owner cannot read can still be written to, and read
    // by others. Its path was just seen as the user's own directory, inside a
    // directory nobody else can change, so it is closed by its path instead
    // and then opened as before.
    fs.chmodSync(dir, 0o700);

    fd = fs.openSync(dir, flags);
  }

  try {
    const stat = fs.fstatSync(fd);

    if (!stat.isDirectory() || stat.uid !== uid) {
      throw new Error(`${dir} is not a directory of the current user`);
    }

    if (stat.dev !== dev) {
      throw new Error(`${dir} is on another file system`);
    }

    // eslint-disable-next-line no-bitwise
    if ((stat.mode & 0o7777) !== 0o700) {
      fs.fchmodSync(fd, 0o700);
    }
  } finally {
    fs.closeSync(fd);
  }
}

/**
 * Check, below a directory already closed to others, that the tree is laid
 * out the way earlier versions staged a report, belongs to the user and stays
 * on one file system. Each directory is closed before what it holds is read,
 * and the whole tree is checked before any of it is removed.
 *
 * @param {string} dir
 * @param {number} uid
 * @param {number} dev - the file system the tree has to stay on
 * @param {number} depth - 0 for the staging directory, 1 for a service's
 */
function checkStagingTree(dir, uid, dev, depth) {
  const samples = depth === 0 ? LEGACY_ROOT_SAMPLES : getLegacyServiceSamples(path.basename(dir));

  for (const name of fs.readdirSync(dir)) {
    const entry = path.join(dir, name);
    const entryStat = fs.lstatSync(entry);

    if (entryStat.dev !== dev) {
      throw new Error(`${entry} is on another file system`);
    }

    if (entryStat.uid !== uid) {
      throw new Error(`${entry} belongs to another user`);
    }

    if (entryStat.isFile() && isSampleFileName(name, samples)) {
      continue;
    }

    if (depth === 0 && entryStat.isDirectory() && LEGACY_SERVICES.includes(name)) {
      closeDirectory(entry, uid, dev);
      checkStagingTree(entry, uid, dev, depth + 1);

      continue;
    }

    throw new Error(`${entry} is not one of the files an earlier report wrote`);
  }
}

/**
 * Read a staging directory's record of when it was collected, refusing a
 * file of any other size than such a record: whatever it holds instead is
 * not read
 *
 * @param {string} datePath
 * @param {number} length - the size of the record expected
 * @return {string|null} the record, or null when there is none of that size
 */
function readCollectionDate(datePath, length) {
  let fd;

  try {
    // eslint-disable-next-line no-bitwise
    fd = fs.openSync(datePath, fs.constants.O_RDONLY | fs.constants.O_NOFOLLOW);
  } catch {
    return null;
  }

  try {
    const stat = fs.fstatSync(fd);

    if (!stat.isFile() || stat.size !== length) {
      return null;
    }

    const buffer = Buffer.alloc(length);

    return fs.readSync(fd, buffer, 0, length, 0) === length ? buffer.toString('utf8') : null;
  } finally {
    fs.closeSync(fd);
  }
}

/**
 * Check that a directory is one an earlier version staged a report in: a
 * real directory of the user's own, on the temp directory's file system,
 * closed to other users before anything in it is read, holding exactly the
 * files earlier versions wrote, and recording the time it is named after.
 *
 * @param {string} dir
 * @param {number} uid
 * @param {number} tempDev
 * @param {{restricted: boolean}} state - set once the directory is closed to
 *   other users, so a caller leaving it in place can say so
 */
function verifyStagingDir(dir, uid, tempDev, state) {
  const name = path.basename(dir);

  // Something mounted in its place is not what an earlier version left
  if (fs.lstatSync(dir).dev !== tempDev) {
    throw new Error(`${dir} is on another file system`);
  }

  // Closed first, so that a directory left in place for any reason below
  // still keeps its samples from other users
  closeDirectory(dir, uid, tempDev);
  // eslint-disable-next-line no-param-reassign
  state.restricted = true;

  checkStagingTree(dir, uid, tempDev, 0);

  const time = name.slice(LEGACY_STAGING_DIR_PREFIX.length);

  if (readCollectionDate(path.join(dir, 'date.txt'), Buffer.byteLength(time)) !== time) {
    throw new Error(`${path.join(dir, 'date.txt')} does not record the time ${name} is named after`);
  }
}

/**
 * Find the directories earlier versions staged reports in and left behind.
 * Nothing is removed: what is found is for the operator to agree to remove.
 *
 * Only real directories under the earlier name that belong to the user
 * running dashmate are looked at. A link under that name is left alone, so
 * nothing it points to is touched, and so is anything another user owns.
 * Nothing is looked at in a temp directory others could swap entries in, and
 * nothing where the operating system has no user ids to compare.
 *
 * Every directory looked at is closed to other users first. One that does
 * not hold exactly what an earlier version staged is left in place.
 *
 * @param {string} tempDir - the directory reports were staged in
 * @return {{
 *   found: string[],
 *   left: {dir: string, error: Error, restricted: boolean}[],
 *   scanFailure: {dir: string, error: Error}|null,
 * }} `restricted` says whether a directory left in place was closed to other
 *   users; `scanFailure` says the temp directory itself could not be looked
 *   in
 */
export function findLegacyReportStagingDirs(tempDir) {
  const found = [];
  const left = [];

  if (typeof process.getuid !== 'function') {
    return { found, left, scanFailure: null };
  }

  const uid = process.getuid();

  let resolvedTempDir;
  let tempDev;
  let names;

  try {
    // With a link before a `..`, the operating system and the path resolver
    // reach different directories from the same path, and the one listed
    // could differ from the one removed from. Only a plain absolute path is
    // used, resolved once by the operating system, and the resolved one is
    // used for everything below.
    if (!path.isAbsolute(tempDir) || path.normalize(tempDir) !== tempDir) {
      throw new Error('the path to it is not a plain absolute path');
    }

    resolvedTempDir = fs.realpathSync.native(tempDir);

    if (!isGuardedPath(resolvedTempDir, uid)) {
      throw new Error('it, or a directory above it, can be changed by other users');
    }

    tempDev = fs.lstatSync(resolvedTempDir).dev;
    names = fs.readdirSync(resolvedTempDir);
  } catch (error) {
    return { found, left, scanFailure: { dir: resolvedTempDir ?? tempDir, error } };
  }

  for (const name of names.filter(isLegacyStagingDirName)) {
    const dir = path.join(resolvedTempDir, name);
    const state = { restricted: false };

    try {
      // Not followed: a link is reported as a link, never as its target
      const stat = fs.lstatSync(dir);

      if (stat.isDirectory() && stat.uid === uid) {
        verifyStagingDir(dir, uid, tempDev, state);

        found.push(dir);
      }
    } catch (error) {
      left.push({ dir, error, restricted: state.restricted });
    }
  }

  return { found, left, scanFailure: null };
}

/**
 * Remove staging directories the operator agreed to remove. Each one is
 * checked again first, so one that changed since it was found is left in
 * place.
 *
 * @param {string[]} dirs - as found by findLegacyReportStagingDirs
 * @return {{removed: string[], failed: {dir: string, error: Error, restricted: boolean}[]}}
 */
export function removeLegacyReportStagingDirs(dirs) {
  const removed = [];
  const failed = [];

  // Nothing is found where ownership cannot be told, so nothing can be agreed to
  if (dirs.length === 0 || typeof process.getuid !== 'function') {
    return { removed, failed };
  }

  const uid = process.getuid();

  for (const dir of dirs) {
    const state = { restricted: false };

    try {
      const stat = fs.lstatSync(dir);

      if (!stat.isDirectory() || stat.uid !== uid) {
        throw new Error(`${dir} is no longer a directory of the current user`);
      }

      verifyStagingDir(dir, uid, fs.lstatSync(path.dirname(dir)).dev, state);

      fs.rmSync(dir, { recursive: true, force: true });

      removed.push(dir);
    } catch (error) {
      failed.push({ dir, error, restricted: state.restricted });
    }
  }

  return { removed, failed };
}
