import HomeDir from '../../../src/config/HomeDir.js';
import getBaseConfigFactory from '../../../configs/defaults/getBaseConfigFactory.js';
import getLocalConfigFactory from '../../../configs/defaults/getLocalConfigFactory.js';

describe('state sync options', () => {
  let getBaseConfig;

  beforeEach(() => {
    getBaseConfig = getBaseConfigFactory(HomeDir.createTemp());
  });

  describe('defaults', () => {
    it('should enable consuming and serving snapshots on the base config', () => {
      const config = getBaseConfig();

      expect(config.get('platform.drive.tenderdash.stateSync')).to.deep.equal({
        enabled: true,
        retries: 3,
        chunkRequestTimeout: '15s',
        fetchersCount: 4,
      });

      expect(config.get('platform.drive.abci.stateSync')).to.deep.equal({
        snapshots: {
          enabled: true,
          frequencySeconds: 600,
          maxCount: 6,
        },
      });
    });

    // A local network genesis starts every node from scratch at the same time,
    // so there is no populated peer to sync from and nothing worth serving.
    it('should disable consuming and serving snapshots on the local preset', () => {
      const config = getLocalConfigFactory(getBaseConfig)();

      expect(config.get('platform.drive.tenderdash.stateSync.enabled')).to.be.false();
      expect(config.get('platform.drive.abci.stateSync.snapshots.enabled')).to.be.false();
    });
  });

  describe('schema', () => {
    let config;

    beforeEach(() => {
      config = getBaseConfig();
    });

    // Tenderdash treats 0 as an unlimited retry count: SyncAny only returns
    // errNoSnapshots (the block sync fallback trigger) when retries > 0.
    it('should accept retries of 0 (retry indefinitely) but not negative', () => {
      config.set('platform.drive.tenderdash.stateSync.retries', 0);

      expect(() => config.set('platform.drive.tenderdash.stateSync.retries', -1))
        .to.throw();
    });

    it('should accept 1 to 64 fetchers only', () => {
      config.set('platform.drive.tenderdash.stateSync.fetchersCount', 1);
      config.set('platform.drive.tenderdash.stateSync.fetchersCount', 64);

      expect(() => config.set('platform.drive.tenderdash.stateSync.fetchersCount', 0))
        .to.throw();
      expect(() => config.set('platform.drive.tenderdash.stateSync.fetchersCount', 65))
        .to.throw();
    });

    // Tenderdash 1.7 rejects a statesync chunk-request-timeout below 5 seconds.
    it('should reject a chunk request timeout below the 5s Tenderdash minimum', () => {
      ['5s', '15s', '1.5m', '0.5m', '2h', '0.1h', '5000ms', '30000ms'].forEach((valid) => {
        config.set('platform.drive.tenderdash.stateSync.chunkRequestTimeout', valid);
      });

      ['0', '4s', '4.9s', '4999ms', '500ms', '0.05m', 'nonsense', 15].forEach((invalid) => {
        expect(() => config.set('platform.drive.tenderdash.stateSync.chunkRequestTimeout', invalid), String(invalid))
          .to.throw();
      });
    });

    // The fractional minute/hour floor is 0.1, which is conservative: 5s is a
    // non-terminating decimal in minutes (0.0833...m), so a regex can't hit the
    // boundary exactly. Durations between 5s and the floor (e.g. 0.09m = 5.4s,
    // 0.01h = 36s) must be spelled in s or ms, which express any duration exactly.
    it('should accept fractional minutes and hours down to 0.1 only', () => {
      ['0.1m', '0.15m', '0.1h'].forEach((valid) => {
        config.set('platform.drive.tenderdash.stateSync.chunkRequestTimeout', valid);
      });

      ['0.09m', '0.01h', '0.099m'].forEach((invalid) => {
        expect(() => config.set('platform.drive.tenderdash.stateSync.chunkRequestTimeout', invalid), invalid)
          .to.throw();
      });
    });

    it('should reject a snapshot frequency below one minute', () => {
      config.set('platform.drive.abci.stateSync.snapshots.frequencySeconds', 60);

      expect(() => config.set('platform.drive.abci.stateSync.snapshots.frequencySeconds', 59))
        .to.throw();
    });

    it('should keep at least two snapshots', () => {
      config.set('platform.drive.abci.stateSync.snapshots.maxCount', 2);

      expect(() => config.set('platform.drive.abci.stateSync.snapshots.maxCount', 1))
        .to.throw();
    });

    it('should reject unknown state sync options', () => {
      expect(() => config.set('platform.drive.tenderdash.stateSync.maxConcurrentListSnapshots', 100))
        .to.throw();
      expect(() => config.set('platform.drive.abci.stateSync.snapshots.frequency', 5))
        .to.throw();
    });
  });
});
