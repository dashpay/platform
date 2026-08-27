import fs from 'fs';
import path from 'path';

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
 * A real write, not a permission bit: a full disk passes every access check and
 * fails the only thing that matters.
 *
 * @param {string[]} directories
 * @return {boolean} false only when a directory that exists refused a write
 */
export default function certificateStorageWritable(directories) {
  return directories.every((directory) => {
    // A directory that is not there yet is not a storage fault. It is what a
    // node that has never held a certificate looks like, and that node needs
    // the obtain rather than a repair.
    if (!fs.existsSync(directory)) {
      return true;
    }

    const probe = path.join(directory, `.dashmate-write-probe-${process.pid}`);

    try {
      fs.writeFileSync(probe, '');

      return true;
    } catch {
      return false;
    } finally {
      try {
        fs.rmSync(probe, { force: true });
      } catch {
        // Leaving an empty probe behind is not worth failing a diagnosis over.
      }
    }
  });
}
