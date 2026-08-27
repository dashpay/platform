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
  /**
   * Repair something here first, then let renewal come back around on its own.
   * Only while the certificate in use still works - there is time to wait.
   */
  WAIT_AFTER_LOCAL_FIX: 'WAIT_AFTER_LOCAL_FIX',
  /**
   * Repair something here first, then ask for a certificate.
   * The one in use is already unusable, so waiting is a live outage.
   */
  OBTAIN_AFTER_LOCAL_FIX: 'OBTAIN_AFTER_LOCAL_FIX',
  /** This provider will not issue again; the operator picks another. */
  SWITCH_PROVIDER: 'SWITCH_PROVIDER',
  /** Asking again costs something and gains nothing. */
  DO_NOT_OBTAIN: 'DO_NOT_OBTAIN',
  /**
   * A certificate obtained now could not be saved. Repair the storage first;
   * asking before that spends one of a weekly handful into the same fault.
   */
  REPAIR_STORAGE: 'REPAIR_STORAGE',
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
function safeActionForRemedy(remedy, isCertificateUsable) {
  if (remedy === REMEDY_CLASS.DO_NOT_RETRY || remedy === REMEDY_CLASS.SUPPORT) {
    return SAFE_ACTION.DO_NOT_OBTAIN;
  }

  if (remedy === REMEDY_CLASS.SWITCH_PROVIDER) {
    return SAFE_ACTION.SWITCH_PROVIDER;
  }

  // A repair the operator has just made needs checking, and asking the
  // authority is the only way to check it: dashmate cannot test its own
  // inbound port 80, because nothing listens there except during a renewal -
  // which is why an external port scan reads closed on a healthy node.
  //
  // Sending them away for an hour to find out whether it worked is how a node
  // stays broken: they leave, they forget, and the certificate expires. A
  // failed attempt costs one of five hourly validations, of which renewal
  // itself uses one; a successful one is the certificate they were after.
  // Neither is the weekly allowance, which is what the withholding cases
  // above protect.
  if (remedy === REMEDY_CLASS.FIX_LOCALLY) {
    return SAFE_ACTION.OBTAIN_AFTER_LOCAL_FIX;
  }

  // Waiting is only ever advised on a signal this repository raised itself,
  // and it stays conditional: once the certificate in use has stopped working,
  // an hour of waiting is a live outage.
  if (remedy === REMEDY_CLASS.WAIT) {
    return isCertificateUsable
      ? SAFE_ACTION.WAIT_AFTER_LOCAL_FIX
      : SAFE_ACTION.OBTAIN_AFTER_LOCAL_FIX;
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
 * @param {boolean} [options.isCertificateUsable] - whether the node still has a
 *   working certificate, which decides whether waiting is affordable
 * @return {{cause: string|null, code: string|null, safeAction: string,
 *   issuanceStatus: string, prerequisites: string[]}}
 */
export default function deriveRenewalGuidance({
  record = null,
  isRecordUnreadable = false,
  hasNoExternalIp = false,
  isCertificateUsable = true,
  isCertificateStorageWritable = null,
}) {
  /**
   * Nothing that would ask the authority may go ahead while the answer cannot
   * be written down. Applied after the ordinary derivation rather than inside
   * it, so it covers every route to a request - including the provider switch,
   * which still has to save what it obtains.
   *
   * @param {string} action
   * @param {RenewalRecord|null} candidate
   * @return {string}
   */
  const withStorageChecked = (action, candidate) => {
    const wouldAsk = action === SAFE_ACTION.OBTAIN
      || action === SAFE_ACTION.OBTAIN_AFTER_LOCAL_FIX
      || action === SAFE_ACTION.SWITCH_PROVIDER;

    // A live answer where the caller has one, and the collected one otherwise.
    // The doctor reads archives from other machines and can only have what was
    // collected; `update` runs on the node itself and asks at the moment it
    // matters.
    const writable = isCertificateStorageWritable ?? candidate?.getStorageWritable() ?? null;

    return wouldAsk && writable === false ? SAFE_ACTION.REPAIR_STORAGE : action;
  };

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
      // Nothing recorded still does not mean the certificate could be kept.
      safeAction: withStorageChecked(SAFE_ACTION.OBTAIN, null),
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
    safeAction: withStorageChecked(
      issuanceStatus === ISSUANCE_STATUS.NONE
        ? safeActionForRemedy(remedy, isCertificateUsable)
        : SAFE_ACTION.DO_NOT_OBTAIN,
      record,
    ),
    issuanceStatus,
    prerequisites,
  };
}
