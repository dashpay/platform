import { expect } from 'chai';
import fs from 'fs';
import path from 'path';
import HomeDir from '../../../src/config/HomeDir.js';
import {
  clearRenewalRecord,
  recordGatewayReloadFailure,
  recordRenewalFailure,
  recordRenewalSuccess,
} from '../../../src/helper/record-renewal-outcome.js';
import RenewalRecordRepository, {
  RENEWAL_RECORD_STATES,
} from '../../../src/ssl/renewalRecord/RenewalRecordRepository.js';
import { RENEWAL_FAILURE_CODES } from '../../../src/ssl/renewal-failure.js';
import LegoArtifactsMissingError from '../../../src/ssl/errors/LegoArtifactsMissingError.js';
import LegoResultNotObservedError from '../../../src/ssl/errors/LegoResultNotObservedError.js';
import LegoDidNotStartError from '../../../src/ssl/errors/LegoDidNotStartError.js';
import RenewalRecord from '../../../src/ssl/renewalRecord/RenewalRecord.js';

const CONFIG_NAME = 'mainnet';
const PROVIDER = 'letsencrypt';

describe('recordRenewalOutcome', () => {
  let homeDir;
  let renewalRecordRepository;

  const read = () => renewalRecordRepository.read(CONFIG_NAME);
  const fail = (error) => recordRenewalFailure({
    renewalRecordRepository, homeDir, configName: CONFIG_NAME, provider: PROVIDER, error,
  });

  beforeEach(() => {
    homeDir = HomeDir.createTemp();
    renewalRecordRepository = new RenewalRecordRepository(homeDir);
  });

  afterEach(() => {
    homeDir.remove();
  });

  it('should create the certificate directory, which a node that never obtained one does not have', () => {
    // The directory is made when a certificate is first saved. A node that has
    // never had one is exactly the node whose renewal is worth recording.
    expect(fs.existsSync(path.dirname(renewalRecordRepository.getPath(CONFIG_NAME)))).to.equal(false);

    recordRenewalSuccess({ renewalRecordRepository, configName: CONFIG_NAME, provider: PROVIDER });

    expect(read().state).to.equal(RENEWAL_RECORD_STATES.PRESENT);
  });

  it('should carry the last success forward through failures, and count them', () => {
    recordRenewalSuccess({ renewalRecordRepository, configName: CONFIG_NAME, provider: PROVIDER });
    const { record: succeeded } = read();

    fail(new Error('urn:ietf:params:acme:error:connection :: timeout'));
    fail(new Error('urn:ietf:params:acme:error:connection :: timeout'));

    const { record } = read();

    expect(record.isFailed()).to.equal(true);
    expect(record.getConsecutiveFailures()).to.equal(2);
    // Success is durable state, not something a reader has to observe in
    // flight: the next attempt is scheduled on the tick after a renewal
    // completes, so the succeeded outcome itself is gone within milliseconds.
    expect(record.getLastSuccessAt().toISOString())
      .to.equal(succeeded.getLastSuccessAt().toISOString());
  });

  it('should reset the failure count once a certificate arrives', () => {
    fail(new Error('urn:ietf:params:acme:error:connection :: timeout'));
    recordRenewalSuccess({ renewalRecordRepository, configName: CONFIG_NAME, provider: PROVIDER });

    expect(read().record.getConsecutiveFailures()).to.equal(0);
  });

  it('should keep a spent issuance recorded when a later, different failure replaces the cause', () => {
    // The failure an hour later is "the certificate file is missing", whose
    // ordinary advice is to obtain one. Doing that spends a second certificate
    // against a limit of five a week to fix a problem that is local. The record
    // holds one outcome, so this fact has to outlive the outcome that produced
    // it or the dangerous advice wins simply by being written last.
    fail(new Error('guidance', { cause: new LegoArtifactsMissingError('/tmp/x.crt') }));

    const spentAt = read().record.toObject().issuanceSpentAt;
    expect(spentAt).to.not.equal(null);

    fail(new Error('ENOENT: no such file or directory'));

    const { record } = read();

    expect(record.getCode()).to.not.equal(RENEWAL_FAILURE_CODES.CERTIFICATE_ISSUED_NOT_SAVED);
    expect(record.toObject().issuanceSpentAt).to.equal(spentAt);
  });

  it('should not inherit the previous provider\'s history when the provider changed', () => {
    // A provider change handed over by the configuration watcher does not clear
    // the record. Carrying the old provider's spent issuance forward would
    // suppress the repair for an unrelated failure on the new one.
    recordRenewalFailure({
      renewalRecordRepository,
      homeDir,
      configName: CONFIG_NAME,
      provider: 'letsencrypt',
      error: new Error('guidance', { cause: new LegoArtifactsMissingError('/tmp/x.crt') }),
    });

    expect(read().record.isIssuanceSpent()).to.equal(true);

    recordRenewalFailure({
      renewalRecordRepository,
      homeDir,
      configName: CONFIG_NAME,
      provider: 'zerossl',
      error: new Error('urn:ietf:params:acme:error:connection :: timeout'),
    });

    const { record } = read();

    expect(record.getProvider()).to.equal('zerossl');
    expect(record.isIssuanceSpent()).to.equal(false);
    expect(record.getConsecutiveFailures()).to.equal(1);
    expect(record.getLastSuccessAt()).to.equal(null);
  });

  it('should keep the no-retry guard when an unread result is replaced by a later failure', () => {
    // The helper ran and nobody read how it finished, so a certificate may
    // already have been issued. An hour later an ordinary cause replaces it,
    // and its ordinary advice is to ask for another one.
    fail(new Error('guidance', { cause: new LegoResultNotObservedError(new Error('gone')) }));

    expect(read().record.isIssuanceUncertain()).to.equal(true);

    fail(new Error('urn:ietf:params:acme:error:connection :: timeout'));

    const { record } = read();

    expect(record.getCode()).to.equal(RENEWAL_FAILURE_CODES.PORT_80_UNREACHABLE);
    expect(record.isIssuanceUncertain()).to.equal(true);
    expect(record.isIssuanceOutstanding()).to.equal(true);
  });

  it('should clear an uncertain issuance once a certificate actually arrives', () => {
    fail(new Error('guidance', { cause: new LegoResultNotObservedError(new Error('gone')) }));

    recordRenewalSuccess({ renewalRecordRepository, configName: CONFIG_NAME, provider: PROVIDER });

    expect(read().record.isIssuanceUncertain()).to.equal(false);
  });

  it('should refuse a date whose derived retry instant cannot be represented', () => {
    // Valid on its own and unusable: readers derive the next attempt from it,
    // and formatting that result throws - which would take the whole diagnosis
    // down rather than one field. An archive can carry such a value.
    const recordPath = renewalRecordRepository.getPath(CONFIG_NAME);
    fs.mkdirSync(path.dirname(recordPath), { recursive: true });
    fs.writeFileSync(recordPath, JSON.stringify({
      provider: PROVIDER,
      outcome: 'failed',
      code: 'UNKNOWN',
      attemptedAt: '+275760-09-13T00:00:00.000Z',
      consecutiveFailures: 1,
    }));

    expect(read().state).to.not.equal(RENEWAL_RECORD_STATES.PRESENT);
  });

  it('should not let an unreadable fence silently permit every superseded writer', () => {
    // A fence that exists and cannot be read is not the same as no fence.
    // Treating it as absent lets every superseded chain through at exactly the
    // moment the guard is needed, so the failure is surfaced instead.
    const generationPath = path.join(
      path.dirname(renewalRecordRepository.getPath(CONFIG_NAME)),
      '.renewal-generation',
    );

    fs.mkdirSync(generationPath, { recursive: true });

    // A real record, so the only thing that can decide the outcome is the
    // fence - passing null would throw on its own and prove nothing.
    const record = RenewalRecord.fromObject({
      provider: PROVIDER,
      outcome: 'failed',
      code: RENEWAL_FAILURE_CODES.PORT_80_UNREACHABLE,
      attemptedAt: new Date().toISOString(),
      consecutiveFailures: 1,
    });

    expect(() => renewalRecordRepository.read(CONFIG_NAME)).to.not.throw();
    expect(() => renewalRecordRepository.write(CONFIG_NAME, record, 1)).to.throw();
  });

  it('should not let a superseded chain describe a node it no longer renews', () => {
    // The old job's callback keeps running after the configuration watcher
    // hands over, and both chains write to the same file. Without a fence the
    // attempt that was replaced overwrites the one that replaced it, and both
    // say letsencrypt so the provider guard cannot see it.
    const superseded = renewalRecordRepository.claimGeneration(CONFIG_NAME);
    const current = renewalRecordRepository.claimGeneration(CONFIG_NAME);

    recordRenewalFailure({
      renewalRecordRepository,
      homeDir,
      configName: CONFIG_NAME,
      provider: PROVIDER,
      code: RENEWAL_FAILURE_CODES.CERTIFICATE_FILE_MISSING,
      generation: current,
    });

    recordRenewalFailure({
      renewalRecordRepository,
      homeDir,
      configName: CONFIG_NAME,
      provider: PROVIDER,
      error: new Error('urn:ietf:params:acme:error:rateLimited :: too many'),
      generation: superseded,
    });

    expect(read().record.getCode()).to.equal(RENEWAL_FAILURE_CODES.CERTIFICATE_FILE_MISSING);
  });

  it('should keep the fence after the record is cleared, so a stale writer cannot recreate it', () => {
    // A fence living only inside the record would not survive its removal: the
    // superseded writer finds nothing on disk, concludes it is first, and
    // resurrects state the current chain deliberately dropped.
    const superseded = renewalRecordRepository.claimGeneration(CONFIG_NAME);

    fail(new Error('urn:ietf:params:acme:error:connection :: timeout'));

    const current = renewalRecordRepository.claimGeneration(CONFIG_NAME);
    clearRenewalRecord({ renewalRecordRepository, configName: CONFIG_NAME, generation: current });

    expect(read().state).to.equal(RENEWAL_RECORD_STATES.ABSENT);

    recordRenewalFailure({
      renewalRecordRepository,
      homeDir,
      configName: CONFIG_NAME,
      provider: PROVIDER,
      error: new Error('urn:ietf:params:acme:error:connection :: timeout'),
      generation: superseded,
    });

    expect(read().state).to.equal(RENEWAL_RECORD_STATES.ABSENT);
  });

  it('should not let a superseded chain delete what the current one recorded', () => {
    const superseded = renewalRecordRepository.claimGeneration(CONFIG_NAME);
    const current = renewalRecordRepository.claimGeneration(CONFIG_NAME);

    recordRenewalFailure({
      renewalRecordRepository,
      homeDir,
      configName: CONFIG_NAME,
      provider: PROVIDER,
      error: new Error('urn:ietf:params:acme:error:connection :: timeout'),
      generation: current,
    });

    clearRenewalRecord({ renewalRecordRepository, configName: CONFIG_NAME, generation: superseded });

    expect(read().state).to.equal(RENEWAL_RECORD_STATES.PRESENT);
  });

  it('should treat a start Docker never confirmed as one that may have spent an issuance', () => {
    // Docker can reject a start it already accepted, so the certificate check
    // may be running and may already have asked the authority.
    fail(new Error('guidance', {
      cause: new LegoDidNotStartError(new Error('connection reset'), false),
    }));

    const { record } = read();

    expect(record.getCode()).to.equal(RENEWAL_FAILURE_CODES.HELPER_START_UNCONFIRMED);
    expect(record.isIssuanceUncertain()).to.equal(true);
  });

  it('should clear a spent issuance once a certificate actually arrives', () => {
    fail(new Error('guidance', { cause: new LegoArtifactsMissingError('/tmp/x.crt') }));

    recordRenewalSuccess({ renewalRecordRepository, configName: CONFIG_NAME, provider: PROVIDER });

    expect(read().record.toObject().issuanceSpentAt).to.equal(null);
  });

  it('should record a failed gateway reload without disturbing the renewal that succeeded', () => {
    recordRenewalSuccess({ renewalRecordRepository, configName: CONFIG_NAME, provider: PROVIDER });

    recordGatewayReloadFailure({ renewalRecordRepository, configName: CONFIG_NAME });

    const { record } = read();

    expect(record.getGatewayReloadFailedAt()).to.not.equal(null);
    // The certificate renewed. Counting this as a renewal failure would tell an
    // operator whose certificate is minutes old that renewal has been failing
    // since whenever it last worked.
    expect(record.isFailed()).to.equal(false);
    expect(record.getConsecutiveFailures()).to.equal(0);
    expect(record.getLastSuccessAt()).to.not.equal(null);
  });

  it('should take the accepted code from the caller when the cause is already known', () => {
    recordRenewalFailure({
      renewalRecordRepository,
      homeDir,
      configName: CONFIG_NAME,
      provider: PROVIDER,
      code: RENEWAL_FAILURE_CODES.CERTIFICATE_FILE_MISSING,
    });

    expect(read().record.getCode()).to.equal(RENEWAL_FAILURE_CODES.CERTIFICATE_FILE_MISSING);
  });

  it('should start over rather than throw when what is already there is corrupt', () => {
    const recordPath = renewalRecordRepository.getPath(CONFIG_NAME);
    fs.mkdirSync(path.dirname(recordPath), { recursive: true });
    fs.writeFileSync(recordPath, '{ this is not json');

    fail(new Error('urn:ietf:params:acme:error:connection :: timeout'));

    const { state, record } = read();

    expect(state).to.equal(RENEWAL_RECORD_STATES.PRESENT);
    expect(record.getConsecutiveFailures()).to.equal(1);
    expect(record.getLastSuccessAt()).to.equal(null);
  });

  it('should not throw when the record cannot be written, so a renewal never fails on bookkeeping', () => {
    // A throw from here reaches the cron callback that owns the renewal chain,
    // where it would skip the stop that schedules the next attempt and leave
    // the helper alive with nothing scheduled and nothing watching.
    const recordPath = renewalRecordRepository.getPath(CONFIG_NAME);
    fs.mkdirSync(recordPath, { recursive: true });

    expect(() => fail(new Error('urn:ietf:params:acme:error:connection :: timeout'))).to.not.throw();
  });

  it('should not throw on anything the renewal might have thrown', () => {
    expect(() => fail(undefined)).to.not.throw();
    expect(() => fail('a string')).to.not.throw();
    expect(read().record.getCode()).to.equal(RENEWAL_FAILURE_CODES.UNKNOWN);
  });

  it('should write a record readable by the account that runs doctor', () => {
    recordRenewalSuccess({ renewalRecordRepository, configName: CONFIG_NAME, provider: PROVIDER });

    // eslint-disable-next-line no-bitwise
    const mode = fs.statSync(renewalRecordRepository.getPath(CONFIG_NAME)).mode & 0o777;

    expect(mode).to.equal(0o644);
  });

  it('should forget the record when renewal stops being this provider\'s concern', () => {
    fail(new Error('urn:ietf:params:acme:error:connection :: timeout'));

    clearRenewalRecord({ renewalRecordRepository, configName: CONFIG_NAME });

    expect(read().state).to.equal(RENEWAL_RECORD_STATES.ABSENT);
  });

  it('should not throw when asked to forget a record that was never written', () => {
    expect(() => clearRenewalRecord({ renewalRecordRepository, configName: CONFIG_NAME })).to.not.throw();
  });

  it('should never store an error object, whose message masking cannot reach', () => {
    // Neither `message` nor `stack` is enumerable, so an error placed in a
    // report is invisible to the masking applied to it: the operator's home
    // directory would travel out intact and arrive as an empty object.
    fail(new Error('urn:ietf:params:acme:error:connection :: timeout'));

    const raw = JSON.parse(fs.readFileSync(renewalRecordRepository.getPath(CONFIG_NAME), 'utf8'));

    Object.values(raw).forEach((value) => {
      expect(value === null || typeof value !== 'object').to.equal(true);
    });
  });
});
