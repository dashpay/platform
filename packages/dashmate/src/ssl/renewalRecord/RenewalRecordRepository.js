import fs from 'fs';
import path from 'path';
import writeFileAtomic from 'write-file-atomic';
import RenewalRecord from './RenewalRecord.js';

/**
 * Whether anything was recorded, and whether it could be used.
 *
 * Absent and unreadable are kept apart deliberately. Reporting a file that
 * could not be opened as "nothing recorded" answers a question this cannot
 * answer, on the node where the answer matters most.
 */
export const RENEWAL_RECORD_STATES = {
  ABSENT: 'ABSENT',
  UNREADABLE: 'UNREADABLE',
  PRESENT: 'PRESENT',
};

/**
 * Readable by the operator's own tooling and by nothing that needs protecting.
 *
 * Nothing secret is written here by construction, so locking the file down
 * would contradict that and add the one failure this design can otherwise
 * avoid: a read refused because the account running `doctor` is not the account
 * that ran `start`. The private key beside it keeps its own mode.
 */
const RECORD_FILE_MODE = 0o644;

export default class RenewalRecordRepository {
  /**
   * @param {HomeDir} homeDir
   */
  constructor(homeDir) {
    this.homeDir = homeDir;
  }

  /**
   * Where the record for one config lives.
   *
   * Beside the certificate it describes: invalidated by the same events,
   * removed by the same reset, and inside a directory the helper already
   * writes to. The gateway mounts `bundle.crt` and `private.key` individually
   * rather than the directory, so nothing here reaches Envoy.
   *
   * @param {string} configName
   * @return {string}
   */
  getPath(configName) {
    return this.homeDir.joinPath(configName, 'platform', 'gateway', 'ssl', 'renewal.json');
  }

  /**
   * Read what the helper recorded about the last renewal for one config.
   *
   * @param {string} configName
   * @return {{state: string, path: string, record: RenewalRecord|null, error: string|null}}
   */
  read(configName) {
    const recordPath = this.getPath(configName);

    let contents;

    try {
      contents = fs.readFileSync(recordPath, 'utf8');
    } catch (e) {
      if (e.code === 'ENOENT') {
        return {
          state: RENEWAL_RECORD_STATES.ABSENT, path: recordPath, record: null, error: null,
        };
      }

      // The message only, never the error. Neither `message` nor `stack` is
      // enumerable, so an error object placed in a collected report is
      // invisible to the masking applied to it - it would carry the operator's
      // home directory out intact and arrive as an empty object at the far end.
      return {
        state: RENEWAL_RECORD_STATES.UNREADABLE,
        path: recordPath,
        record: null,
        error: String(e.message),
      };
    }

    let parsed;

    try {
      parsed = JSON.parse(contents);
    } catch (e) {
      return {
        state: RENEWAL_RECORD_STATES.UNREADABLE,
        path: recordPath,
        record: null,
        error: String(e.message),
      };
    }

    const record = RenewalRecord.fromObject(parsed);

    if (record === null) {
      return {
        state: RENEWAL_RECORD_STATES.UNREADABLE,
        path: recordPath,
        record: null,
        error: 'The renewal record does not describe a renewal outcome',
      };
    }

    return {
      state: RENEWAL_RECORD_STATES.PRESENT, path: recordPath, record, error: null,
    };
  }

  /**
   * @param {string} configName
   * @param {RenewalRecord} record
   * @return {void}
   */
  write(configName, record) {
    const recordPath = this.getPath(configName);

    // The directory belongs to the certificate and is created when one is first
    // saved, so a node that has never obtained one does not have it yet - which
    // is exactly the node whose renewal is worth recording.
    fs.mkdirSync(path.dirname(recordPath), { recursive: true });

    // Replaced by rename, so a reader never sees half a record. Safe here only
    // because nothing mounts this file: the certificate beside it is
    // bind-mounted into the gateway individually and has to be written in place.
    writeFileAtomic.sync(
      recordPath,
      `${JSON.stringify(record.toObject(), undefined, 2)}\n`,
      { encoding: 'utf8', mode: RECORD_FILE_MODE },
    );
  }

  /**
   * Forget what was recorded for this config.
   *
   * Used when renewal stops being a provider's concern - SSL turned off, or a
   * provider switch - and when a certificate is installed by hand, which
   * settles any failure that came before it.
   *
   * @param {string} configName
   * @return {void}
   */
  remove(configName) {
    fs.rmSync(this.getPath(configName), { force: true });
  }
}
