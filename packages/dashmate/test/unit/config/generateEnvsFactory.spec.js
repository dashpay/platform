import { expect } from 'chai';
import HomeDir from '../../../src/config/HomeDir.js';
import getBaseConfigFactory from '../../../configs/defaults/getBaseConfigFactory.js';
import generateEnvsFactory from '../../../src/config/generateEnvsFactory.js';
import getConfigProfilesFactory from '../../../src/config/getConfigProfilesFactory.js';

describe('generateEnvsFactory', () => {
  let config;
  let generateEnvs;

  beforeEach(() => {
    const homeDir = HomeDir.createTemp();
    config = getBaseConfigFactory(homeDir)();

    const configFile = { getProjectId: () => null };
    generateEnvs = generateEnvsFactory(configFile, homeDir, getConfigProfilesFactory());
  });

  it('should not include the Tor compose file when Tor is disabled', () => {
    config.set('core.tor.enabled', false);

    const envs = generateEnvs(config);

    expect(envs.COMPOSE_FILE.split(':')).to.not.include('docker-compose.tor.yml');
  });

  it('should include the Tor compose file and image by default', () => {
    const envs = generateEnvs(config);

    expect(envs.COMPOSE_FILE.split(':')).to.include('docker-compose.tor.yml');
    expect(envs.CORE_TOR_DOCKER_IMAGE).to.equal(config.get('core.tor.docker.image'));
  });

  it('should pin the default Tor image by digest', () => {
    // The image is published by a third party under a mutable tag.
    expect(config.get('core.tor.docker.image')).to.match(/^osminogin\/tor-simple:[0-9.]+@sha256:[0-9a-f]{64}$/);
  });
});
