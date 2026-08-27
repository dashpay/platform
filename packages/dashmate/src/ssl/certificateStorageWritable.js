import fs from 'fs';
import path from 'path';

/**
 * Roughly a certificate chain and a key. Small enough not to matter, large
 * enough that a filesystem with no data blocks left refuses it - which a
 * zero-byte create does not.
 */
const PROBE_BYTES = 8 * 1024;

/**
 * Whether a certificate obtained now could actually be saved.
 *
 * The one case the issuance markers cannot cover. When the helper obtains a
 * certificate and then fails to write it, it exits non-zero, and dashmate only
 * records that an issuance happened on a clean exit - so nothing marks the
 * allowance as spent. The next failure overwrites the record, and an operator
 * who repairs the original cause and asks again spends a second certificate
 * from a weekly handful into the same broken storage.
 *
 * Asked directly rather than inferred, because nothing local distinguishes
 * "issued, then could not be saved" from "never issued" - the difference lives
 * only in text that is partly the responder's to write.
 *
 * Two kinds of target, because a renewal writes both: new files in the helper's
 * own directory, and the two files the gateway already has bind-mounted, which
 * are overwritten in place rather than replaced.
 *
 * @param {Object} options
 * @param {string[]} [options.directories] - must accept a new file
 * @param {string[]} [options.files] - must accept being overwritten, if present
 * @return {boolean} false only when something that exists refused
 */
export default function certificateStorageWritable({ directories = [], files = [] }) {
  const acceptsNewFile = (directory) => {
    // A directory that is not there yet is not a storage fault. It is what a
    // node that has never held a certificate looks like, and that node needs
    // the request rather than a repair.
    if (!fs.existsSync(directory)) {
      return true;
    }

    const probe = path.join(directory, `.dashmate-write-probe-${process.pid}`);

    try {
      // Written with content, not created empty. The fault that matters passes
      // every permission check: a full disk is writable by mode, accepts an
      // empty inode, and refuses the bytes.
      fs.writeFileSync(probe, Buffer.alloc(PROBE_BYTES, 0x20));

      return true;
    } catch {
      return false;
    } finally {
      try {
        fs.rmSync(probe, { force: true });
      } catch {
        // Leaving a probe behind is not worth failing a diagnosis over.
      }
    }
  };

  const acceptsOverwrite = (file) => {
    if (!fs.existsSync(file)) {
      return true;
    }

    // Opened for update rather than truncated: this asks whether the gateway's
    // own certificate could be replaced without destroying the one it is
    // serving. A file owned by another user, or on a read-only mount, refuses
    // here while its directory still accepts new files.
    let handle;

    try {
      handle = fs.openSync(file, 'r+');

      return true;
    } catch {
      return false;
    } finally {
      if (handle !== undefined) {
        try {
          fs.closeSync(handle);
        } catch {
          // Nothing was changed; a failed close cannot invalidate the answer.
        }
      }
    }
  };

  return directories.every(acceptsNewFile) && files.every(acceptsOverwrite);
}
