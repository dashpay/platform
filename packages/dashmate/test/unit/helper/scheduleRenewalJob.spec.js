import { expect } from 'chai';
import HomeDir from '../../../src/config/HomeDir.js';
import scheduleRenewalJob from '../../../src/helper/scheduleRenewalJob.js';
import ServiceIsNotRunningError from '../../../src/docker/errors/ServiceIsNotRunningError.js';
import readRenewalRecord, {
  RENEWAL_OUTCOMES,
  RENEWAL_RECORD_STATES,
} from '../../../src/ssl/renewalRecord.js';
import { RENEWAL_FAILURE_CODES } from '../../../src/ssl/renewalFailure.js';

const CONFIG_NAME = 'base';
const PROVIDER = 'letsencrypt';

/**
 * The job owns the renewal chain: the certificate, the signal to the gateway,
 * the record, and the stop that arms the next attempt. None of it had any
 * assertion, so every ordering guarantee here held by reading alone.
 */
describe('scheduleRenewalJob', () => {
  let homeDir;
  let config;
  let configFileRepository;
  let dockerCompose;
  let rescheduledWith;
  let configurationChangedWith;
  let retryDelay;
  let realSetTimeout;

  const read = () => readRenewalRecord(homeDir, CONFIG_NAME);

  /**
   * Drive one firing of the job and wait for it to settle.
   *
   * @param {Object} options
   * @return {Promise<void>}
   */
  async function run({ obtainError = null, execError = null } = {}) {
    dockerCompose.execCommand = async () => {
      if (execError) {
        throw execError;
      }
    };

    scheduleRenewalJob({
      // Soon, but not already past - cron refuses a fire time behind it.
      renewAt: new Date(Date.now() + 60),
      currentConfig: config,
      provider: PROVIDER,
      providerName: "Let's Encrypt",
      expirationDays: 2,
      obtainCertificateTask: () => ({
        run: async () => {
          if (obtainError) {
            throw obtainError;
          }
        },
      }),
      configFileRepository,
      writeConfigTemplates: () => {},
      dockerCompose,
      homeDir,
      onConfigurationChanged: async (changed) => {
        configurationChangedWith.push(changed);
      },
      reschedule: (next) => {
        rescheduledWith.push(next);
      },
    });

    // Let the cron tick, the awaits inside it, and the onComplete all drain.
    await new Promise((resolve) => {
      realSetTimeout(resolve, 250);
    });
  }

  beforeEach(function beforeEach() {
    homeDir = HomeDir.createTemp();
    rescheduledWith = [];
    configurationChangedWith = [];
    retryDelay = null;
    realSetTimeout = setTimeout;

    // The hourly retry must not actually wait an hour, but whether it was armed
    // at all is the thing under test.
    this.sinon.stub(global, 'setTimeout').callsFake((fn, ms) => {
      if (ms === 60 * 60 * 1000) {
        retryDelay = ms;

        return 0;
      }

      return realSetTimeout(fn, ms);
    });

    config = {
      getName: () => CONFIG_NAME,
      get: (key) => ({
        'platform.gateway.ssl.enabled': true,
        'platform.gateway.ssl.provider': PROVIDER,
        externalIp: '198.51.100.7',
      })[key],
      isChanged: () => false,
    };

    configFileRepository = {
      acquire: () => {},
      release: () => {},
      isExclusive: () => true,
      write: () => {},
      read: () => ({ getConfig: () => config }),
      readAndMigrate: () => ({ configFile: { getConfig: () => config } }),
    };

    dockerCompose = { execCommand: async () => {} };
  });

  afterEach(() => {
    homeDir.remove();
  });

  it('should record a renewal as succeeded and reschedule rather than retry', async () => {
    await run();

    const { record } = read();

    expect(record.outcome).to.equal(RENEWAL_OUTCOMES.SUCCEEDED);
    expect(record.consecutiveFailures).to.equal(0);
    expect(record.lastSuccessAt).to.be.a('string');
    expect(rescheduledWith).to.have.lengthOf(1);
    expect(retryDelay).to.equal(null);
  });

  it('should record a failed renewal with the cause the provider gave', async () => {
    await run({
      obtainError: new Error('acme: error: 400 :: urn:ietf:params:acme:error:connection :: timeout'),
    });

    const { record } = read();

    expect(record.outcome).to.equal(RENEWAL_OUTCOMES.FAILED);
    expect(record.code).to.equal(RENEWAL_FAILURE_CODES.PORT_80_UNREACHABLE);
    expect(record.consecutiveFailures).to.equal(1);
  });

  it('should arm the hourly retry before the record is written', async () => {
    // `job.stop()` is the only thing that fires the completion handler that
    // arms the retry. A record write ahead of it that throws would leave the
    // helper alive with no scheduled attempt and no configuration watcher -
    // renewal dead until the container restarts, and nothing said about it.
    //
    // Making the write impossible is how the ordering is observed: if the write
    // ran first and escaped, the retry below would never have been armed.
    const { mkdirSync } = await import('fs');
    mkdirSync(`${homeDir.getPath()}/${CONFIG_NAME}/platform/gateway/ssl/renewal.json`, {
      recursive: true,
    });

    await run({ obtainError: new Error('renewal failed') });

    expect(retryDelay).to.equal(60 * 60 * 1000);
    expect(read().state).to.not.equal(RENEWAL_RECORD_STATES.PRESENT);
  });

  it('should not count a gateway that is merely stopped as a failed renewal', async () => {
    // The certificate renewed. A gateway that is down is not a certificate
    // problem, it is already reported as a stopped service, and the documented
    // upgrade procedure leaves it down on purpose.
    await run({ execError: new ServiceIsNotRunningError(CONFIG_NAME, 'gateway') });

    const { record } = read();

    expect(record.outcome).to.equal(RENEWAL_OUTCOMES.SUCCEEDED);
    expect(record.gatewayReloadFailedAt).to.equal(null);
    expect(record.consecutiveFailures).to.equal(0);
  });

  it('should record a failed signal as a reload failure, never as a failed renewal', async () => {
    // Folding the two together would tell an operator whose certificate renewed
    // minutes ago that renewal had been failing since their previous one.
    await run({ execError: new Error('container exec failed') });

    const { record } = read();

    expect(record.outcome).to.equal(RENEWAL_OUTCOMES.SUCCEEDED);
    expect(record.gatewayReloadFailedAt).to.be.a('string');
    expect(record.consecutiveFailures).to.equal(0);
    expect(record.lastSuccessAt).to.be.a('string');
  });

  it('should forget the record before handing renewal to another provider', async () => {
    // The handover is awaited, and the provider taking over writes its own
    // first record inside it. Clearing afterwards would delete that record and
    // leave a switched node reporting nothing until its next attempt.
    let recordDuringHandover;

    // Seeded, so that finding nothing during the handover is a fact about the
    // clear rather than a fact about an empty directory.
    const { recordRenewalFailure } = await import('../../../src/helper/recordRenewalOutcome.js');
    recordRenewalFailure({
      homeDir, configName: CONFIG_NAME, provider: PROVIDER, error: new Error('stale'),
    });
    expect(read().state).to.equal(RENEWAL_RECORD_STATES.PRESENT);

    config.get = (key) => ({
      // What renewCertificate re-reads under the lock: this provider no longer
      // owns renewal here.
      'platform.gateway.ssl.enabled': true,
      'platform.gateway.ssl.provider': 'zerossl',
      externalIp: '198.51.100.7',
    })[key];

    scheduleRenewalJob({
      renewAt: new Date(Date.now() + 60),
      currentConfig: { ...config, getName: () => CONFIG_NAME },
      provider: PROVIDER,
      providerName: "Let's Encrypt",
      expirationDays: 2,
      obtainCertificateTask: () => ({ run: async () => {} }),
      configFileRepository,
      writeConfigTemplates: () => {},
      dockerCompose,
      homeDir,
      onConfigurationChanged: async () => {
        recordDuringHandover = read().state;
      },
      reschedule: () => {},
    });

    await new Promise((resolve) => {
      realSetTimeout(resolve, 250);
    });

    expect(recordDuringHandover).to.equal(RENEWAL_RECORD_STATES.ABSENT);
  });
});
