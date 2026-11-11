import fs from 'node:fs';
import getConfiguredDockerImagesFactory from '../../../src/docker/getConfiguredDockerImagesFactory.js';
import generateEnvsFactory from '../../../src/config/generateEnvsFactory.js';
import getConfigProfilesFactory from '../../../src/config/getConfigProfilesFactory.js';
import getBaseConfigFactory from '../../../configs/defaults/getBaseConfigFactory.js';
import HomeDir from '../../../src/config/HomeDir.js';
import { DASHMATE_HELPER_DOCKER_IMAGE } from '../../../src/constants.js';

describe('getConfiguredDockerImagesFactory', () => {
  let homeDir;
  let config;
  let otherConfig;
  let getImages;
  let dynamicComposePath;

  beforeEach(() => {
    homeDir = HomeDir.createTemp();
    config = getBaseConfigFactory()();
    otherConfig = getBaseConfigFactory()();
    otherConfig.set('platform.drive.abci.docker.image', 'dashpay/drive:other');
    const configFile = {
      getProjectId: () => null,
      getAllConfigs: () => [config, otherConfig],
    };
    getImages = getConfiguredDockerImagesFactory(
      configFile,
      generateEnvsFactory(configFile, homeDir, getConfigProfilesFactory()),
    );
    fs.mkdirSync(homeDir.joinPath('base'));
    dynamicComposePath = homeDir.joinPath('base', 'dynamic-compose.yml');
  });

  afterEach(() => homeDir.remove());

  it('should include all configs, disabled services, helper and locally built image references', () => {
    config.set('platform.enable', false);
    config.set('core.insight.enabled', false);
    const references = getImages();

    expect(references.has('dashpay/drive:other')).to.be.true();
    expect(references.has('dashpay/insight-api:latest')).to.be.true();
    expect(references.has('redis:alpine')).to.be.true();
    expect(references.has('drive:local')).to.be.true();
    expect(references.has('rs-dapi:local')).to.be.true();
    expect(references.has(DASHMATE_HELPER_DOCKER_IMAGE)).to.be.true();
  });

  it('should protect literal and environment image overrides from generated Compose files', () => {
    fs.writeFileSync(dynamicComposePath, 'services:\n  custom:\n    image: dashpay/drive:custom\n'
      // Compose interpolation, not a JavaScript template expression.
      // eslint-disable-next-line no-template-curly-in-string
      + '  another:\n    image: ${CORE_DOCKER_IMAGE:?err}\n');

    const references = getImages();

    expect(references.has('dashpay/drive:custom')).to.be.true();
    expect(references.has(config.get('core.docker.image'))).to.be.true();
  });

  it('should fail closed on unsupported interpolation rather than lose a protected image', () => {
    // eslint-disable-next-line no-template-curly-in-string
    fs.writeFileSync(dynamicComposePath, 'services:\n  custom:\n    image: dashpay/drive:${VERSION}\n');

    expect(getImages).to.throw('Cannot safely resolve');
  });

  it('should fail closed on invalid YAML rather than lose a protected image', () => {
    fs.writeFileSync(dynamicComposePath, 'services: [');

    expect(getImages).to.throw();
  });
});
