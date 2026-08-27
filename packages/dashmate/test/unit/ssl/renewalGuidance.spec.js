import { expect } from 'chai';
import deriveRenewalGuidance, { ISSUANCE_STATUS, SAFE_ACTION } from '../../../src/ssl/renewalGuidance.js';
import RenewalRecord from '../../../src/ssl/renewalRecord/RenewalRecord.js';
import { RENEWAL_FAILURE_CODES } from '../../../src/ssl/renewal-failure.js';

/**
 * @param {string} code
 * @param {Object} [overrides]
 * @return {RenewalRecord}
 */
function failed(code, overrides = {}) {
  return RenewalRecord.fromObject({
    provider: 'letsencrypt',
    outcome: 'failed',
    code,
    attemptedAt: new Date().toISOString(),
    consecutiveFailures: 1,
    ...overrides,
  });
}

describe('deriveRenewalGuidance', () => {
  it('should offer the check whether or not the certificate still works', () => {
    // A repair has been described and the operator needs to know whether it
    // took. There is nothing they can probe: port 80 has no listener outside a
    // renewal. A failed check costs one of five hourly validations, which is
    // not the allowance worth guarding - the weekly one is, and only an
    // outstanding issuance or a spent provider quota can waste that.
    const record = failed(RENEWAL_FAILURE_CODES.PORT_80_UNREACHABLE);

    expect(deriveRenewalGuidance({ record, isCertificateUsable: true }).safeAction)
      .to.equal(SAFE_ACTION.OBTAIN_AFTER_LOCAL_FIX);
    expect(deriveRenewalGuidance({ record, isCertificateUsable: false }).safeAction)
      .to.equal(SAFE_ACTION.OBTAIN_AFTER_LOCAL_FIX);
  });

  // Stated over the whole set rather than case by case. A per-case list is what
  // let a rate limit quietly choose "wait" and an unfamiliar problem type
  // quietly choose "support", each of which stops an operator repairing a port
  // they could have opened.
  it('should give every cause read from a message the same action', () => {
    const messageDerived = [
      RENEWAL_FAILURE_CODES.PORT_80_UNREACHABLE,
      RENEWAL_FAILURE_CODES.PORT_80_WRONG_RESPONDER,
      RENEWAL_FAILURE_CODES.RATE_LIMITED,
      RENEWAL_FAILURE_CODES.CERTIFICATE_CHECK_REFUSED,
    ];

    [true, false].forEach((isCertificateUsable) => {
      const actions = messageDerived.map((code) => deriveRenewalGuidance({
        record: failed(code),
        isCertificateUsable,
      }).safeAction);

      expect(actions).to.deep.equal(
        Array(messageDerived.length).fill(SAFE_ACTION.OBTAIN_AFTER_LOCAL_FIX),
      );
    });
  });

  // The gap the issuance markers cannot cover. A helper that obtains a
  // certificate and then fails to save it exits non-zero, and nothing records
  // that an issuance happened - so the operator who repairs the original cause
  // is offered a request that spends a second weekly certificate into the same
  // broken storage.
  it('should withhold every request while the certificate could not be saved', () => {
    [
      RENEWAL_FAILURE_CODES.PORT_80_UNREACHABLE,
      RENEWAL_FAILURE_CODES.RATE_LIMITED,
      RENEWAL_FAILURE_CODES.QUOTA_EXHAUSTED,
    ].forEach((code) => {
      const record = failed(code, { storageWritable: false });

      expect(deriveRenewalGuidance({ record }).safeAction)
        .to.equal(SAFE_ACTION.REPAIR_STORAGE);
    });
  });

  // Only an outright refusal withholds. A record written before this was
  // checked carries nothing, and must not be read as an assurance either way.
  it('should not withhold when storage was never asked about', () => {
    [undefined, true].forEach((storageWritable) => {
      const record = failed(RENEWAL_FAILURE_CODES.PORT_80_UNREACHABLE, { storageWritable });

      expect(deriveRenewalGuidance({ record }).safeAction)
        .to.equal(SAFE_ACTION.OBTAIN_AFTER_LOCAL_FIX);
    });
  });

  it('should let an outstanding issuance outrank a cause that could otherwise be repaired', () => {
    const record = failed(RENEWAL_FAILURE_CODES.PORT_80_UNREACHABLE, {
      issuanceSpentAt: new Date().toISOString(),
    });

    const guidance = deriveRenewalGuidance({ record, isCertificateUsable: false });

    expect(guidance.safeAction).to.equal(SAFE_ACTION.DO_NOT_OBTAIN);
    expect(guidance.issuanceStatus).to.equal(ISSUANCE_STATUS.SPENT);
  });

  it('should keep an unread result apart from a confirmed spend', () => {
    const record = failed(RENEWAL_FAILURE_CODES.RESULT_UNKNOWN, {
      issuanceUncertainAt: new Date().toISOString(),
    });

    expect(deriveRenewalGuidance({ record }).issuanceStatus)
      .to.equal(ISSUANCE_STATUS.UNCERTAIN);
  });

  it('should refuse to spend anything on evidence it could not read', () => {
    const guidance = deriveRenewalGuidance({ isRecordUnreadable: true });

    expect(guidance.safeAction).to.equal(SAFE_ACTION.DO_NOT_OBTAIN);
    expect(guidance.issuanceStatus).to.equal(ISSUANCE_STATUS.UNCERTAIN);
  });

  it('should carry the address prerequisite whatever the cause says', () => {
    expect(deriveRenewalGuidance({ hasNoExternalIp: true }).prerequisites)
      .to.contain('EXTERNAL_IP');
  });
});
