import fs from 'fs';
import path from 'path';
import writeFileAtomic from 'write-file-atomic';
import classifyRenewalFailure from '../ssl/renewalFailure.js';
import readRenewalRecord, {
  clearRenewalRecord as removeRenewalRecord,
  RENEWAL_OUTCOMES,
  RENEWAL_RECORD_FORMAT_VERSION,
  RENEWAL_RECORD_STATES,
  renewalRecordPath,
} from '../ssl/renewalRecord.js';

/**
 * Readable by the operator's own tooling and by nothing that needs protecting.
 *
 * Nothing secret is written here by construction, so locking the file down
 * would contradict that and add the one failure this design can otherwise
 * avoid: a read refused because the account running `doctor` is not the
 * account that ran `start`. The private key beside it keeps its own mode.
 */
const RECORD_FILE_MODE = 0o644;

/**
 * Everything that can throw, kept inside one boundary.
 *
 * This runs from the cron callback that owns the renewal chain, where an
 * escaping error would take down the process and, worse, skip the stop that
 * schedules the next attempt. Renewal must never fail because its bookkeeping
 * did, so classification, serialisation and the write are all in here and
 * nothing is evaluated by the caller on the way in.
 *
 * @param {function(): void} write
 * @param {string} configName
 */
function attempt(write, configName) {
  try {
    write();
  } catch (e) {
    // eslint-disable-next-line no-console
    console.error(`Failed to record the certificate renewal outcome for ${configName}: ${e.message}`);
  }
}

/**
 * @param {HomeDir} homeDir
 * @param {string} configName
 * @return {Object|null}
 */
function readPrevious(homeDir, configName) {
  const { state, record } = readRenewalRecord(homeDir, configName);

  return state === RENEWAL_RECORD_STATES.PRESENT ? record : null;
}

/**
 * @param {HomeDir} homeDir
 * @param {string} configName
 * @param {Object} record
 */
function save(homeDir, configName, record) {
  const recordPath = renewalRecordPath(homeDir, configName);

  // The directory belongs to the certificate and is created when one is first
  // saved, so a node that has never obtained one does not have it yet - which
  // is exactly the node whose renewal is worth recording.
  fs.mkdirSync(path.dirname(recordPath), { recursive: true });

  // Replaced by rename, so a reader never sees half a record. Safe here only
  // because nothing mounts this file: the certificate beside it is bind-mounted
  // into the gateway individually and has to be written in place instead.
  writeFileAtomic.sync(
    recordPath,
    `${JSON.stringify({ formatVersion: RENEWAL_RECORD_FORMAT_VERSION, ...record }, undefined, 2)}\n`,
    { encoding: 'utf8', mode: RECORD_FILE_MODE },
  );
}

/**
 * Record that a renewal completed.
 *
 * Written before the gateway is told to load the certificate, because the two
 * are separate facts. Folding a failed signal into this one would carry the
 * previous success forward and count a failure, and an operator whose
 * certificate renewed minutes ago would be told renewal had been failing for
 * as long as the old date is old.
 *
 * @param {Object} options
 * @param {HomeDir} options.homeDir
 * @param {string} options.configName
 * @param {string} options.provider
 */
export function recordRenewalSuccess({ homeDir, configName, provider }) {
  attempt(() => {
    save(homeDir, configName, {
      provider,
      outcome: RENEWAL_OUTCOMES.SUCCEEDED,
      attemptedAt: new Date().toISOString(),
      lastSuccessAt: new Date().toISOString(),
      consecutiveFailures: 0,
      // A certificate arrived, so whatever was owed from an earlier attempt is
      // settled and the warning against asking for another one is lifted.
      issuanceSpentAt: null,
      gatewayReloadFailedAt: null,
    });
  }, configName);
}

/**
 * Record that a renewal did not produce a certificate.
 *
 * @param {Object} options
 * @param {HomeDir} options.homeDir
 * @param {string} options.configName
 * @param {string} options.provider
 * @param {*} [options.error] - classified here, never by the caller
 * @param {string} [options.code] - when the caller already knows the cause
 * @param {string} [options.apiKey] - redacted defensively out of the excerpt
 */
export function recordRenewalFailure({
  homeDir, configName, provider, error, code, apiKey,
}) {
  attempt(() => {
    const previous = readPrevious(homeDir, configName);
    const classified = code
      ? { code, detail: null }
      : classifyRenewalFailure(error, { homeDirPath: homeDir.getPath(), apiKey });

    const issuanceSpentAt = classified.code === 'CERTIFICATE_ISSUED_NOT_SAVED'
      ? new Date().toISOString()
      // Carried until a certificate actually arrives. An issuance that was
      // spent and never landed stays true through every later failure, and the
      // next attempt an hour from now records a different cause whose ordinary
      // advice is to ask for another certificate - which is the one thing that
      // must not happen while this is set.
      : previous?.issuanceSpentAt ?? null;

    save(homeDir, configName, {
      provider,
      outcome: RENEWAL_OUTCOMES.FAILED,
      code: classified.code,
      detail: classified.detail,
      attemptedAt: new Date().toISOString(),
      lastSuccessAt: previous?.lastSuccessAt ?? null,
      consecutiveFailures: (previous?.consecutiveFailures ?? 0) + 1,
      issuanceSpentAt,
      gatewayReloadFailedAt: null,
    });
  }, configName);
}

/**
 * Record that a renewed certificate could not be handed to the gateway.
 *
 * Kept apart from the renewal's own outcome: the certificate is on disk and
 * the counter and the last success stay as the renewal left them. Only the
 * loading of it failed, and only that is repaired.
 *
 * @param {Object} options
 * @param {HomeDir} options.homeDir
 * @param {string} options.configName
 */
export function recordGatewayReloadFailure({ homeDir, configName }) {
  attempt(() => {
    const previous = readPrevious(homeDir, configName);

    if (previous === null) {
      return;
    }

    save(homeDir, configName, {
      ...previous,
      gatewayReloadFailedAt: new Date().toISOString(),
    });
  }, configName);
}

/**
 * Forget what was recorded for this config.
 *
 * Used when renewal stops being this provider's concern - SSL turned off, or
 * a provider switch - and when a certificate is installed by hand, which
 * settles any failure that came before it. Left behind, a failure record for a
 * node whose operator deliberately stopped renewing would be reported forever.
 *
 * @param {Object} options
 * @param {HomeDir} options.homeDir
 * @param {string} options.configName
 */
export function clearRenewalRecord({ homeDir, configName }) {
  attempt(() => removeRenewalRecord(homeDir, configName), configName);
}
