import getBaseConfigFactory from '../../../../configs/defaults/getBaseConfigFactory.js';
import analyseSystemResourcesFactory from '../../../../src/doctor/analyse/analyseSystemResourcesFactory.js';
import verifySystemRequirementsFactory from '../../../../src/doctor/verifySystemRequirementsFactory.js';
import Samples from '../../../../src/doctor/Samples.js';

describe('analyseSystemResourcesFactory', () => {
  let analyseSystemResources;
  let config;
  let samples;

  beforeEach(() => {
    config = getBaseConfigFactory()();

    samples = new Samples();
    samples.setDashmateConfig(config);

    // 12GB clears the doctor's 5GB base disk requirement on its own,
    // but not with the 10GB snapshot headroom on top
    samples.setSystemInfo({
      diskSpace: { available: 12 * 1024 ** 3 },
    });

    analyseSystemResources = analyseSystemResourcesFactory(
      verifySystemRequirementsFactory(),
    );
  });

  describe('state sync snapshot disk headroom', () => {
    it('should apply headroom when Platform and snapshots are enabled', () => {
      config.set('platform.enable', true);
      config.set('platform.drive.abci.stateSync.snapshots.enabled', true);

      const problems = analyseSystemResources(samples);

      expect(problems).to.have.lengthOf(1);
      expect(problems[0].getDescription())
        .to.include('At least 15GB is required (including 10GB headroom for state sync snapshots)');
    });

    it('should not apply headroom when Platform is disabled', () => {
      // A fullnode/masternode setup disables Platform but keeps the base
      // snapshot default of true; Drive isn't running to create snapshots
      config.set('platform.enable', false);
      config.set('platform.drive.abci.stateSync.snapshots.enabled', true);

      const problems = analyseSystemResources(samples);

      expect(problems).to.be.empty();
    });

    it('should not apply headroom when snapshots are disabled', () => {
      config.set('platform.enable', true);
      config.set('platform.drive.abci.stateSync.snapshots.enabled', false);

      const problems = analyseSystemResources(samples);

      expect(problems).to.be.empty();
    });
  });
});
