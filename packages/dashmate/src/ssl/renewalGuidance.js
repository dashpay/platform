import { describeRenewalFailure, REMEDY_CLASS } from './renewal-failure.js';

/**
 * Whether asking the authority for another certificate is safe right now.
 *
 * Derived once and read by every surface. Both operator surfaces previously
 * worked this out for themselves from the raw record, and they drifted apart
 * three times: one would withhold a command while the other printed it, for
 * the same node in the same state. Precedence belongs in one place.
 */
export const SAFE_ACTION = {
  /** Ask for a certificate. Nothing known forbids it. */
  OBTAIN: 'OBTAIN',
  /** Repair something here first; renewal comes back around on its own. */
  FIX_LOCALLY_THEN_WAIT: 'FIX_LOCALLY_THEN_WAIT',
  /** This provider will not issue again; the operator picks another. */
  SWITCH_PROVIDER: 'SWITCH_PROVIDER',
  /** Asking again costs something and gains nothing. */
  DO_NOT_OBTAIN: 'DO_NOT_OBTAIN',
};

/**
 * What is known about whether an issuance is already outstanding.
 *
 * Three states, not two. "May have been issued" and "was issued and could not
 * be saved" withhold the same command for different reasons, and telling an
 * operator their certificate could not be saved when dashmate does not know
 * whether one exists is a claim it cannot make.
 */
export const ISSUANCE_STATUS = {
  NONE: 'NONE',
  UNCERTAIN: 'UNCERTAIN',
  SPENT: 'SPENT',
};

/**
 * @param {string} remedy
 * @return {string}
 */
function safeActionForRemedy(remedy) {
  if (remedy === REMEDY_CLASS.DO_NOT_RETRY || remedy === REMEDY_CLASS.SUPPORT) {
    return SAFE_ACTION.DO_NOT_OBTAIN;
  }

  if (remedy === REMEDY_CLASS.SWITCH_PROVIDER) {
    return SAFE_ACTION.SWITCH_PROVIDER;
  }

  // Transient, and renewal retries by itself - so asking now spends an attempt
  // on a condition that has not changed yet.
  if (remedy === REMEDY_CLASS.FIX_LOCALLY || remedy === REMEDY_CLASS.WAIT) {
    return SAFE_ACTION.FIX_LOCALLY_THEN_WAIT;
  }

  return SAFE_ACTION.OBTAIN;
}

/**
 * Everything an operator surface needs to say, decided once.
 *
 * @param {Object} options
 * @param {RenewalRecord|null} options.record - applicable and failed, or null
 * @param {boolean} [options.isRecordUnreadable] - a record exists and could not
 *   be read, so nothing about issuance can be established either way
 * @param {boolean} [options.hasNoExternalIp] - nothing can be issued without an
 *   address, so this outranks every other prerequisite
 * @return {{cause: string|null, code: string|null, safeAction: string,
 *   issuanceStatus: string, prerequisites: string[]}}
 */
export default function deriveRenewalGuidance({
  record = null,
  isRecordUnreadable = false,
  hasNoExternalIp = false,
}) {
  const prerequisites = hasNoExternalIp ? ['EXTERNAL_IP'] : [];

  // Nothing can be established, so nothing may be spent on the strength of it.
  if (isRecordUnreadable) {
    return {
      cause: null,
      code: null,
      safeAction: SAFE_ACTION.DO_NOT_OBTAIN,
      issuanceStatus: ISSUANCE_STATUS.UNCERTAIN,
      prerequisites,
    };
  }

  if (record === null) {
    return {
      cause: null,
      code: null,
      safeAction: SAFE_ACTION.OBTAIN,
      issuanceStatus: ISSUANCE_STATUS.NONE,
      prerequisites,
    };
  }

  const { sentence, remedy } = describeRenewalFailure(record.getCode());

  let issuanceStatus = ISSUANCE_STATUS.NONE;

  if (record.isIssuanceSpent()) {
    issuanceStatus = ISSUANCE_STATUS.SPENT;
  } else if (record.isIssuanceUncertain()) {
    issuanceStatus = ISSUANCE_STATUS.UNCERTAIN;
  }

  return {
    cause: sentence,
    code: record.getCode(),
    // An outstanding issuance outranks the cause's own remedy: it is spent, or
    // may be, whether or not this particular failure could be repaired.
    safeAction: issuanceStatus === ISSUANCE_STATUS.NONE
      ? safeActionForRemedy(remedy)
      : SAFE_ACTION.DO_NOT_OBTAIN,
    issuanceStatus,
    prerequisites,
  };
}
