import classifyRenewalFailure, { RENEWAL_FAILURE_CODES } from '../ssl/renewalFailure.js';
import RenewalRecord from '../ssl/renewalRecord/RenewalRecord.js';
import { RENEWAL_RECORD_STATES } from '../ssl/renewalRecord/RenewalRecordRepository.js';

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
 * @param {RenewalRecordRepository} repository
 * @param {string} configName
 * @return {RenewalRecord|null}
 */
function readPrevious(repository, configName) {
  const { state, record } = repository.read(configName);

  return state === RENEWAL_RECORD_STATES.PRESENT ? record : null;
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
 * @param {RenewalRecordRepository} options.renewalRecordRepository
 * @param {string} options.configName
 * @param {string} options.provider
 */
export function recordRenewalSuccess({ renewalRecordRepository, configName, provider }) {
  attempt(() => {
    // One instant for both, so they cannot order against each other.
    const now = new Date().toISOString();

    renewalRecordRepository.write(configName, RenewalRecord.fromObject({
      provider,
      outcome: RenewalRecord.OUTCOMES.SUCCEEDED,
      attemptedAt: now,
      lastSuccessAt: now,
      consecutiveFailures: 0,
      // A certificate arrived, so whatever was owed from an earlier attempt is
      // settled and the warning against asking for another one is lifted.
      issuanceSpentAt: null,
      gatewayReloadFailedAt: null,
    }));
  }, configName);
}

/**
 * Record that a renewal did not produce a certificate.
 *
 * @param {Object} options
 * @param {RenewalRecordRepository} options.renewalRecordRepository
 * @param {HomeDir} options.homeDir
 * @param {string} options.configName
 * @param {string} options.provider
 * @param {*} [options.error] - classified here, never by the caller
 * @param {string} [options.code] - when the caller already knows the cause
 * @param {string} [options.apiKey] - redacted defensively out of the excerpt
 */
export function recordRenewalFailure({
  renewalRecordRepository, homeDir, configName, provider, error, code, apiKey,
}) {
  attempt(() => {
    // Only this provider's own history. A provider change handed over by the
    // configuration watcher does not clear the record, so without this the new
    // provider's first failure would inherit the old one's last success, its
    // failure count, and its spent issuance - and a certificate spent on one
    // provider would suppress the repair for an unrelated failure on another.
    const candidate = readPrevious(renewalRecordRepository, configName);
    const previous = candidate?.getProvider() === provider ? candidate : null;

    const classified = code
      ? { code, detail: null }
      : classifyRenewalFailure(error, { homeDirPath: homeDir.getPath(), apiKey });

    const spentBefore = previous?.isIssuanceSpent() ? previous.toObject().issuanceSpentAt : null;

    const issuanceSpentAt = classified.code === RENEWAL_FAILURE_CODES.CERTIFICATE_ISSUED_NOT_SAVED
      ? new Date().toISOString()
      // Carried until a certificate actually arrives. An issuance that was
      // spent and never landed stays true through every later failure, and the
      // next attempt an hour from now records a different cause whose ordinary
      // advice is to ask for another certificate - which is the one thing that
      // must not happen while this is set.
      : spentBefore;

    renewalRecordRepository.write(configName, RenewalRecord.fromObject({
      provider,
      outcome: RenewalRecord.OUTCOMES.FAILED,
      code: classified.code,
      detail: classified.detail,
      attemptedAt: new Date().toISOString(),
      lastSuccessAt: previous?.getLastSuccessAt()?.toISOString() ?? null,
      consecutiveFailures: (previous?.getConsecutiveFailures() ?? 0) + 1,
      issuanceSpentAt,
      gatewayReloadFailedAt: null,
    }));
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
 * @param {RenewalRecordRepository} options.renewalRecordRepository
 * @param {string} options.configName
 */
export function recordGatewayReloadFailure({ renewalRecordRepository, configName }) {
  attempt(() => {
    const previous = readPrevious(renewalRecordRepository, configName);

    if (previous === null) {
      return;
    }

    renewalRecordRepository.write(configName, RenewalRecord.fromObject({
      ...previous.toObject(),
      gatewayReloadFailedAt: new Date().toISOString(),
    }));
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
 * @param {RenewalRecordRepository} options.renewalRecordRepository
 * @param {string} options.configName
 */
export function clearRenewalRecord({ renewalRecordRepository, configName }) {
  attempt(() => renewalRecordRepository.remove(configName), configName);
}
