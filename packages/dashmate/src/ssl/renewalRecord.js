import fs from 'fs';
import { sanitizeDetail } from './renewalFailure.js';

/**
 * The shape this build writes.
 *
 * Recorded for a person opening the file, and for a future reader that has a
 * reason to care. Nothing here gates on it: a reader that refused an unfamiliar
 * version would go silent on a node that is actively failing, which is the one
 * outcome worse than reporting nothing at all. Fields are validated one at a
 * time instead, so an unfamiliar shape degrades to the parts that are
 * recognisable.
 */
export const RENEWAL_RECORD_FORMAT_VERSION = 1;

export const RENEWAL_RECORD_STATES = {
  /** Nothing has been recorded here. Not a fault on its own. */
  ABSENT: 'ABSENT',
  /** Something is there and could not be used. Never reported as absent. */
  UNREADABLE: 'UNREADABLE',
  PRESENT: 'PRESENT',
};

export const RENEWAL_OUTCOMES = {
  SUCCEEDED: 'succeeded',
  FAILED: 'failed',
};

/**
 * Where the record for one config lives.
 *
 * Beside the certificate it describes: invalidated by the same events, removed
 * by the same reset, and inside the directory the helper already writes to.
 * The gateway mounts `bundle.crt` and `private.key` individually rather than
 * the directory, so nothing here reaches Envoy.
 *
 * @param {HomeDir} homeDir
 * @param {string} configName
 * @return {string}
 */
export function renewalRecordPath(homeDir, configName) {
  return homeDir.joinPath(configName, 'platform', 'gateway', 'ssl', 'renewal.json');
}

/**
 * @param {*} value
 * @return {string|null}
 */
function readTimestamp(value) {
  if (typeof value !== 'string') {
    return null;
  }

  const parsed = new Date(value);

  return Number.isNaN(parsed.getTime()) ? null : parsed.toISOString();
}

/**
 * @param {*} value
 * @return {number}
 */
function readCounter(value) {
  return Number.isInteger(value) && value >= 0 ? value : 0;
}

/**
 * Keep the fields that are recognisable and drop the rest.
 *
 * A record can be written by a newer dashmate, or edited by hand. Taking it
 * field by field means an unfamiliar or damaged value costs only itself,
 * rather than discarding an account of a failure that is otherwise sound.
 *
 * @param {Object} raw
 * @return {Object|null}
 */
function validate(raw) {
  if (raw === null || typeof raw !== 'object' || Array.isArray(raw)) {
    return null;
  }

  const outcome = Object.values(RENEWAL_OUTCOMES).includes(raw.outcome) ? raw.outcome : null;
  const attemptedAt = readTimestamp(raw.attemptedAt);

  // Without these two nothing can be said: no verdict, and no moment to judge
  // it against or to compare with the certificate on disk.
  if (outcome === null || attemptedAt === null) {
    return null;
  }

  return {
    provider: typeof raw.provider === 'string' ? raw.provider : null,
    outcome,
    code: typeof raw.code === 'string' ? raw.code : null,
    // Sanitised again on the way in. The file is editable by hand and a report
    // can arrive from someone else, so what was safe when written is not
    // established at the point it is rendered.
    detail: typeof raw.detail === 'string' ? sanitizeDetail(raw.detail) || null : null,
    attemptedAt,
    lastSuccessAt: readTimestamp(raw.lastSuccessAt),
    consecutiveFailures: readCounter(raw.consecutiveFailures),
    issuanceSpentAt: readTimestamp(raw.issuanceSpentAt),
    gatewayReloadFailedAt: readTimestamp(raw.gatewayReloadFailedAt),
  };
}

/**
 * Read what the helper recorded about the last renewal for one config.
 *
 * Absent and unreadable are kept apart on purpose. Reporting a file that could
 * not be opened as "nothing recorded" would answer a question this cannot
 * answer, on the node where the answer matters most.
 *
 * @param {HomeDir} homeDir
 * @param {string} configName
 * @return {{state: string, path: string, record: Object|null, error: string|null}}
 */
export default function readRenewalRecord(homeDir, configName) {
  const recordPath = renewalRecordPath(homeDir, configName);
  const absent = {
    state: RENEWAL_RECORD_STATES.ABSENT, path: recordPath, record: null, error: null,
  };

  let contents;

  try {
    contents = fs.readFileSync(recordPath, 'utf8');
  } catch (e) {
    if (e.code === 'ENOENT') {
      return absent;
    }

    return {
      state: RENEWAL_RECORD_STATES.UNREADABLE,
      path: recordPath,
      record: null,
      // The message only, never the error. Neither `message` nor `stack` is
      // enumerable, so an error object placed in a report is invisible to the
      // masking applied to it - it would carry the operator's home directory
      // out intact and arrive as an empty object at the other end.
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

  const record = validate(parsed);

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
 * Whether a record still describes the certificate this node is using.
 *
 * Two ways it stops doing so. A provider switch leaves the previous provider's
 * account behind, and it says nothing about the one now in use. And a
 * certificate obtained by hand after a failure overtakes that failure
 * completely - the helper does not notice, because it stops watching the
 * configuration while it waits to retry and the value it watches does not
 * change when a certificate is installed. Without this an operator who has
 * just repaired their node is told renewal is failing, at the exact moment
 * they run the command to check.
 *
 * @param {Object|null} record
 * @param {Object} options
 * @param {string} options.provider - the configured provider
 * @param {Date|null} [options.certificateValidFrom] - of the installed certificate
 * @return {boolean}
 */
export function isRenewalRecordCurrent(record, { provider, certificateValidFrom = null }) {
  if (record === null) {
    return false;
  }

  if (record.provider !== provider) {
    return false;
  }

  if (record.outcome !== RENEWAL_OUTCOMES.FAILED || certificateValidFrom === null) {
    return true;
  }

  return new Date(record.attemptedAt).getTime() > certificateValidFrom.getTime();
}

/**
 * Forget what was recorded for this config.
 *
 * Used when renewal stops being a provider's concern - SSL turned off, or a
 * provider switch - and when a certificate is installed by hand, which settles
 * any failure that came before it. Left behind, a failure recorded against a
 * node whose operator has since repaired it would be reported as current, at
 * the moment they run the command to check their work.
 *
 * @param {HomeDir} homeDir
 * @param {string} configName
 * @return {void}
 */
export function clearRenewalRecord(homeDir, configName) {
  fs.rmSync(renewalRecordPath(homeDir, configName), { force: true });
}
