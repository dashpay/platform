import HomeDir from '../../../src/config/HomeDir.js';
import ConfigFile from '../../../src/config/configFile/ConfigFile.js';
import getBaseConfigFactory from '../../../configs/defaults/getBaseConfigFactory.js';
import getMainnetConfigFactory from '../../../configs/defaults/getMainnetConfigFactory.js';
import generateEnvsFactory from '../../../src/config/generateEnvsFactory.js';
import getConfigProfilesFactory from '../../../src/config/getConfigProfilesFactory.js';
import getServiceListFactory from '../../../src/docker/getServiceListFactory.js';

describe('getServiceListFactory', () => {
  let homeDir;
  let config;
  let getServiceList;

  beforeEach(() => {
    homeDir = HomeDir.createTemp();

    const getBaseConfig = getBaseConfigFactory(homeDir);
    const getMainnetConfig = getMainnetConfigFactory(homeDir, getBaseConfig);

    // Mainnet's defaults have Platform disabled.
    config = getMainnetConfig();
    config.set('platform.gateway.rateLimiter.enabled', true);
    config.set('platform.gateway.rateLimiter.metrics.enabled', true);

    const configFile = new ConfigFile([getBaseConfig(), config], '4.1.0', 'abcdef12', 'mainnet', null);
    const getConfigProfiles = getConfigProfilesFactory();
    const generateEnvs = generateEnvsFactory(configFile, homeDir, getConfigProfiles);

    getServiceList = getServiceListFactory(generateEnvs, getConfigProfiles);
  });

  afterEach(() => {
    homeDir.remove();
  });

  // The exporter only serves the gateway's rate limiter. On a node without
  // Platform it has nothing to export and must not be started, pulled or
  // reported, and Platform-wide operations find Platform services by profile.
  it('should not list the rate limiter metrics exporter when Platform is disabled', () => {
    const serviceNames = getServiceList(config).map(({ name }) => name);

    expect(serviceNames).to.include('core');
    expect(serviceNames).to.not.include('gateway_rate_limiter_metrics');
  });

  it('should list the rate limiter metrics exporter as a Platform service', () => {
    config.set('platform.enable', true);

    const exporter = getServiceList(config)
      .find(({ name }) => name === 'gateway_rate_limiter_metrics');

    expect(exporter.profiles).to.deep.equal(['platform']);
  });

  // A Platform reset on a node with Platform disabled still has to reach the
  // Platform services the node ran before.
  it('should list services of profiles the config does not enable when asked to', () => {
    const serviceNames = getServiceList(config, { allProfiles: true }).map(({ name }) => name);

    expect(serviceNames).to.include.members(['core', 'drive_abci', 'drive_tenderdash', 'quorum_list', 'gateway_rate_limiter_metrics']);
  });
});
