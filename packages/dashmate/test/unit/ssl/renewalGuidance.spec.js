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
  it('should let a working node wait, and send a broken one to obtain', () => {
    // The same cause, and the right answer differs - which is precisely why
    // neither surface may decide it alone. Waiting costs nothing while the
    // certificate works, and is a live outage once it does not.
    const record = failed(RENEWAL_FAILURE_CODES.PORT_80_UNREACHABLE);

    expect(deriveRenewalGuidance({ record, isCertificateUsable: true }).safeAction)
      .to.equal(SAFE_ACTION.WAIT_AFTER_LOCAL_FIX);
    expect(deriveRenewalGuidance({ record, isCertificateUsable: false }).safeAction)
      .to.equal(SAFE_ACTION.OBTAIN_AFTER_LOCAL_FIX);
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
