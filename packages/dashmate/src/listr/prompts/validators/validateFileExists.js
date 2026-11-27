import fs from 'fs';

/**
 * Whether a file exists and this process can read it
 *
 * Its callers read a certificate or key and write a copy of their own, so
 * write access is not needed: a key hardened to 0400, or files on a read-only
 * mount, are accepted.
 *
 * @param {string} value
 * @returns {boolean}
 */
export default function validateFileExists(value) {
  try {
    fs.accessSync(value, fs.constants.R_OK);

    return true;
  } catch (e) {
    return false;
  }
}
