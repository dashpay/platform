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
 * Make sure nobody else can change a staging directory tree while it is
 * removed, and that all of it is the user's own and on one file system.
 *
 * Each directory is opened without following links, checked and closed to
 * other users through that handle, so a link put in its place is never
 * followed, and only then is what it holds read. Nothing in a closed
 * directory can be renamed or replaced afterwards, and the whole tree is
 * checked before any of it is removed.
 *
 * @param {string} dir
 * @param {number} uid
 * @param {number} dev - the file system the tree has to stay on
 */
function lockDownTree(dir, uid, dev) {
  // eslint-disable-next-line no-bitwise
  const fd = fs.openSync(dir, fs.constants.O_RDONLY | fs.constants.O_NOFOLLOW | fs.constants.O_DIRECTORY);

  try {
    const stat = fs.fstatSync(fd);

    if (!stat.isDirectory() || stat.uid !== uid) {
      throw new Error(`${dir} is not a directory of the current user`);
    }

    if (stat.dev !== dev) {
      throw new Error(`${dir} is on another file system`);
    }

    // eslint-disable-next-line no-bitwise
    const closedMode = stat.mode & 0o7777 & ~0o022;

    // eslint-disable-next-line no-bitwise
    if ((stat.mode & 0o7777) !== closedMode) {
      fs.fchmodSync(fd, closedMode);
    }
  } finally {
    fs.closeSync(fd);
  }

  for (const name of fs.readdirSync(dir)) {
    const entry = path.join(dir, name);
    const entryStat = fs.lstatSync(entry);

    if (entryStat.dev !== dev) {
      throw new Error(`${entry} is on another file system`);
    }

    if (entryStat.uid !== uid) {
      throw new Error(`${entry} belongs to another user`);
    }

    if (entryStat.isDirectory()) {
      lockDownTree(entry, uid, dev);
    }
  }
}

/**
 * Remove the directories earlier versions staged reports in and left behind.
 *
 * Only real directories under the earlier name that belong to the user
 * running dashmate are removed. A link under that name is left alone, so
 * nothing it points to is touched, and so is anything another user owns.
 * Nothing is removed from a temp directory others could swap entries in, and
 * nothing where the operating system has no user ids to compare.
 *
 * @param {string} tempDir - the directory reports were staged in
 * @return {{removed: string[], failed: {dir: string, error: Error}[]}}
 */
export default function removeLegacyReportStagingDirs(tempDir) {
  const removed = [];
  const failed = [];

  if (typeof process.getuid !== 'function') {
    return { removed, failed };
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
    return { removed, failed: [{ dir: resolvedTempDir ?? tempDir, error }] };
  }

  for (const name of names.filter(isLegacyStagingDirName)) {
    const dir = path.join(resolvedTempDir, name);

    try {
      // Not followed: a link is reported as a link, never as its target
      const stat = fs.lstatSync(dir);

      if (stat.isDirectory() && stat.uid === uid) {
        // Something mounted in its place is not what an earlier version left
        if (stat.dev !== tempDev) {
          throw new Error(`${dir} is on another file system`);
        }

        lockDownTree(dir, uid, tempDev);

        fs.rmSync(dir, { recursive: true, force: true });

        removed.push(dir);
      }
    } catch (error) {
      failed.push({ dir, error });
    }
  }

  return { removed, failed };
}
