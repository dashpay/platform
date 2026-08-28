import fs from 'fs';
import path from 'path';
import writeFileAtomic from 'write-file-atomic';
import { randomUUID } from 'crypto';
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

/**
 * How long to wait for another holder before giving up. Every holder keeps the
 * fence for a handful of filesystem calls, so contention clears quickly.
 */
const LOCK_ACQUIRE_TIMEOUT_MS = 5000;

const LOCK_RETRY_INTERVAL_MS = 10;

/**
 * Whether the process that wrote this lock is still running.
 *
 * `process.kill(pid, 0)` sends no signal; it asks the kernel whether the
 * process exists and may be signalled. EPERM means it exists and belongs to
 * someone else, which is still alive for this purpose.
 *
 * An unreadable or unrecognisable lock is treated as dead: it cannot be
 * attributed to anyone, and leaving it would block every later renewal from
 * recording anything at all.
 *
 * @param {string} lockPath
 * @return {boolean}
 */
function isHolderAlive(lockPath) {
  const [pid] = fs.readFileSync(lockPath, 'utf8').split('.');
  const holder = Number.parseInt(pid, 10);

  if (!Number.isInteger(holder) || holder <= 0) {
    return false;
  }

  try {
    process.kill(holder, 0);

    return true;
  } catch (e) {
    return e.code === 'EPERM';
  }
}

export default class RenewalRecordRepository {
  /**
   * The high-water generation, kept beside the record and never removed with it.
   *
   * A fence that lived only inside the record would not survive the record
   * being cleared: a superseded writer would find nothing on disk, conclude it
   * was first, and recreate state the current chain had deliberately dropped.
   */
  #generationPath(configName) {
    return this.homeDir.joinPath(configName, 'platform', 'gateway', 'ssl', '.renewal-generation');
  }

  /**
   * @param {string} configName
   * @return {number}
   */
  /**
   * Run fn with the fence held, so a read and the write it authorises cannot be
   * separated by another process.
   *
   * Reading the high-water mark and then acting on it is only a guard if
   * nothing can claim in between. Two processes reading the same number and
   * both claiming it, or a superseded holder resuming after a newer one has
   * written, would each pass a check that was true when it was made and false
   * by the time it mattered - and the configuration lock does not cover this,
   * because a renewal releases that before its bookkeeping runs.
   *
   * @param {string} configName
   * @param {function(): *} fn
   * @return {*}
   */
  #fenced(configName, fn) {
    const generationPath = this.#generationPath(configName);

    fs.mkdirSync(path.dirname(generationPath), { recursive: true });

    const { token, release } = this.#acquire(generationPath);
    const lockPath = `${generationPath}.lock`;

    // Checked immediately before every mutation, not only when the lock was
    // taken. A holder suspended for longer than the stale threshold has its
    // lock reclaimed and another process may already have written newer state;
    // when it resumes it must not overwrite that. Holding the lock at the start
    // says nothing about holding it at the moment of the write.
    const stillOurs = () => {
      try {
        return fs.readFileSync(lockPath, 'utf8') === token;
      } catch {
        return false;
      }
    };

    try {
      if (!fs.existsSync(generationPath)) {
        // Created under the lock, not before it: two processes reaching an
        // unclaimed fence would otherwise both find it absent and both write
        // zero, and the loser would claim a generation already taken.
        if (!stillOurs()) {
          return false;
        }

        writeFileAtomic.sync(generationPath, '0\n', { encoding: 'utf8', mode: RECORD_FILE_MODE });
      }

      return fn(stillOurs);
    } finally {
      try {
        release();
      } catch {
        // Releasing reports when the lock was already broken as stale. Nothing
        // thrown here may replace the outcome the caller actually needs.
      }
    }
  }

  /**
   * Take the fence, waiting out a holder rather than failing on first contention.
   *
   * An exclusive create rather than a lock library: this runs on the helper's
   * only thread, inside a cron callback, and in tests that replace the global
   * timers - so a fence that depends on a timer to stay alive is a fence that
   * can fail for reasons having nothing to do with renewal. An exclusive
   * create needs no timer and no refresh.
   *
   * @param {string} generationPath
   * @return {{token: string, release: function}}
   */
  #acquire(generationPath) {
    const lockPath = `${generationPath}.lock`;
    const deadline = Date.now() + LOCK_ACQUIRE_TIMEOUT_MS;

    for (;;) {
      try {
        // The holder writes who it is. Without that, a holder whose lock was
        // broken as stale still releases on its way out and deletes whatever
        // lock is there by then - which is the new holder's, leaving two
        // processes believing they hold the same fence.
        const token = `${process.pid}.${randomUUID()}`;
        const handle = fs.openSync(lockPath, 'wx');

        try {
          fs.writeFileSync(handle, token);
        } finally {
          fs.closeSync(handle);
        }

        const release = () => {
          try {
            // Read-then-remove, which is not atomic. It matters far less now
            // that a lock is only reclaimed from a process that no longer
            // exists: a holder that reaches this line is running, so nothing
            // has taken its lock away, and the contender it could once have
            // raced does not exist.
            if (fs.readFileSync(lockPath, 'utf8') === token) {
              fs.rmSync(lockPath, { force: true });
            }
          } catch {
            // Already gone, or taken over. Either way it is not ours to
            // remove, and nothing thrown here may replace the caller's outcome.
          }
        };

        return { token, release };
      } catch (e) {
        if (e.code !== 'EEXIST') {
          throw e;
        }

        // Reclaimed only from a holder that is gone, never from one that is
        // merely slow. A lease measured by age alone takes the lock away from a
        // process that is still running and still about to write - it resumes,
        // finds its ownership check has nothing to say about the moment of the
        // write, and overwrites state a newer holder has already committed.
        // Asking whether the recorded process still exists answers the question
        // the lease was approximating, and a holder that no longer exists
        // cannot resume to overwrite anything.
        try {
          if (!isHolderAlive(lockPath)) {
            fs.rmSync(lockPath, { force: true });

            continue;
          }
        } catch {
          continue;
        }

        if (Date.now() >= deadline) {
          throw new Error(`Timed out waiting for the renewal fence at '${lockPath}'`);
        }

        // Synchronous by necessity: there is no event loop to yield to here,
        // and every holder keeps the fence for a handful of filesystem calls.
        Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, LOCK_RETRY_INTERVAL_MS);
      }
    }
  }

  #readGeneration(configName) {
    let contents;

    try {
      contents = fs.readFileSync(this.#generationPath(configName), 'utf8');
    } catch (e) {
      // No fence yet is the ordinary first-run case and means nobody has been
      // superseded. Any other failure means the fence exists and cannot be
      // read, which is not the same thing - treating it as absent would let
      // every superseded writer through exactly when the guard is needed.
      if (e.code === 'ENOENT') {
        return 0;
      }

      throw e;
    }

    const parsed = Number.parseInt(contents, 10);

    // A fence that exists and cannot be understood is not an absent one.
    // Reading it as zero is what an absent fence reads as, so it would let
    // every superseded writer through at exactly the moment it is needed.
    if (!Number.isInteger(parsed) || parsed < 0) {
      throw new Error(`The renewal generation at '${this.#generationPath(configName)}' is not a`
        + ' number, so dashmate cannot tell which renewal attempt is current');
    }

    return parsed;
  }

  /**
   * Take the next generation, making every earlier holder superseded.
   *
   * Claimed once per scheduling chain, and again by a certificate installed by
   * hand - whoever acts now outranks an attempt still in flight from before.
   *
   * @param {string} configName
   * @return {number}
   */
  claimGeneration(configName) {
    return this.#fenced(configName, (stillOurs) => {
      const next = this.#readGeneration(configName) + 1;

      if (!stillOurs()) {
        throw new Error('The renewal fence was taken over while this claim was in progress');
      }

      writeFileAtomic.sync(
        this.#generationPath(configName),
        `${next}\n`,
        { encoding: 'utf8', mode: RECORD_FILE_MODE },
      );

      return next;
    });
  }

  /**
   * Whether a holder of this generation may still write.
   *
   * @param {string} configName
   * @param {number|null} generation
   * @return {boolean}
   */
  #isCurrent(configName, generation) {
    // An unfenced caller is one that predates the fence or has no chain of its
    // own; it is not superseded by anything.
    if (generation === null || generation === undefined) {
      return true;
    }

    return generation >= this.#readGeneration(configName);
  }

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
   * @param {number|null} [generation] - refuses the write when superseded
   * @return {boolean} whether the write was applied
   */
  write(configName, record, generation = null) {
    return this.#fenced(configName, (stillOurs) => {
      // A superseded chain must not describe a node it no longer renews. Its
      // configuration changed under it, and the chain that took over has
      // already written what is true now.
      if (!this.#isCurrent(configName, generation) || !stillOurs()) {
        return false;
      }

      return this.#writeRecord(configName, record);
    });
  }

  /**
   * @param {string} configName
   * @param {RenewalRecord} record
   * @return {boolean}
   */
  #writeRecord(configName, record) {
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

    return true;
  }

  /**
   * Forget what was recorded for this config.
   *
   * Used when renewal stops being a provider's concern - SSL turned off, or a
   * provider switch - and when a certificate is installed by hand, which
   * settles any failure that came before it.
   *
   * @param {string} configName
   * @param {number|null} [generation] - refuses the removal when superseded
   * @return {boolean} whether the removal was applied
   */
  remove(configName, generation = null) {
    return this.#fenced(configName, (stillOurs) => {
      if (!this.#isCurrent(configName, generation) || !stillOurs()) {
        return false;
      }

      fs.rmSync(this.getPath(configName), { force: true });

      return true;
    });
  }
}
