import getServiceListFactory from '../../../src/docker/getServiceListFactory.js';
import getConfigMock from '../../../src/test/mock/getConfigMock.js';
import { DASHMATE_HELPER_DOCKER_IMAGE } from '../../../src/constants.js';

describe('getServiceList', () => {
  let config;
  let getConfigProfiles;

  const images = {
    CORE_DOCKER_IMAGE: 'dashpay/dashd:23',
    PLATFORM_DRIVE_ABCI_DOCKER_IMAGE: 'dashpay/drive:4',
    PLATFORM_DAPI_RS_DAPI_DOCKER_IMAGE: 'dashpay/rs-dapi:4',
  };

  /**
   * @param {string[]} buildComposeFiles
   * @param {Object} sinon
   * @return {getServiceList}
   */
  function createGetServiceList(buildComposeFiles, sinon) {
    const generateEnvs = sinon.stub().returns({
      COMPOSE_FILE: ['docker-compose.yml', ...buildComposeFiles].join(':'),
      ...images,
    });

    return getServiceListFactory(generateEnvs, getConfigProfiles);
  }

  beforeEach(function it() {
    config = getConfigMock(this.sinon);

    getConfigProfiles = this.sinon.stub().returns(['core', 'platform', 'platform-dapi-rs']);
  });

  it('should not mark any service as built locally by default', function it() {
    const services = createGetServiceList([], this.sinon)(config);

    expect(services).to.have.length.greaterThan(0);
    expect(services.every((service) => service.isBuiltLocally === false)).to.be.true();

    const core = services.find((service) => service.name === 'core');

    expect(core.image).to.equal('dashpay/dashd:23');
  });

  it('should mark Drive as built locally when it is built from sources', function it() {
    const services = createGetServiceList(
      ['docker-compose.build.drive_abci.yml'],
      this.sinon,
    )(config);

    const driveAbci = services.find((service) => service.name === 'drive_abci');
    const core = services.find((service) => service.name === 'core');

    // The build compose file replaces the registry image with a locally built one
    expect(driveAbci.image).to.equal('drive:local');
    expect(driveAbci.isBuiltLocally).to.be.true();

    expect(core.isBuiltLocally).to.be.false();
  });

  it('should mark DAPI as built locally when it is built from sources', function it() {
    const services = createGetServiceList(
      ['docker-compose.build.rs-dapi.yml'],
      this.sinon,
    )(config);

    const rsDapi = services.find((service) => service.name === 'rs_dapi');

    expect(rsDapi.image).to.equal('rs-dapi:local');
    expect(rsDapi.isBuiltLocally).to.be.true();
  });

  it('should mark the helper as built locally while still reporting its released image', function it() {
    const services = createGetServiceList(
      ['docker-compose.build.dashmate_helper.yml'],
      this.sinon,
    )(config);

    const helper = services.find((service) => service.name === 'dashmate_helper');

    expect(helper.isBuiltLocally).to.be.true();

    // The helper is always reported with its released image, while compose runs
    // the locally built `dashmate-helper:local`. Nothing pulls the reported
    // image because the service is built, so the two never disagree in practice
    expect(helper.image).to.equal(DASHMATE_HELPER_DOCKER_IMAGE);
  });

  it('should mark every service built from sources when all builds are enabled', function it() {
    const services = createGetServiceList(
      [
        'docker-compose.build.dashmate_helper.yml',
        'docker-compose.build.drive_abci.yml',
        'docker-compose.build.rs-dapi.yml',
      ],
      this.sinon,
    )(config);

    const builtServices = services
      .filter((service) => service.isBuiltLocally)
      .map((service) => service.name);

    expect(builtServices).to.have.members(['dashmate_helper', 'drive_abci', 'rs_dapi']);

    const core = services.find((service) => service.name === 'core');

    expect(core.isBuiltLocally).to.be.false();
  });
});
