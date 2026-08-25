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
