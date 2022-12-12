const getServicesScopeFactory = require('../../../src/status/scopes/services');

describe('services scope unit tests', () => {
  let mockDockerCompose;

  let config;
  let getServicesScope;

  beforeEach(async function it() {
    mockDockerCompose = {inspectService: this.sinon.stub()};

    config = {get: this.sinon.stub(), toEnvs: this.sinon.stub()};
    getServicesScope = getServicesScopeFactory(mockDockerCompose);
  });

  it.only('should just work', async () => {
    mockDockerCompose.inspectService.resolves({
      Id: 'fakeId',
      State: {
        Status: 'running',
      },
      Config: {
        Image: 'fakeImageId',
      },
    })

    const scope = await getServicesScope(config)

    for (const [, service] of Object.entries(scope)) {
      expect(service.containerId).to.be.equal('fakeId')
      expect(service.image).to.be.equal('fakeImageId')
      expect(service.status).to.be.equal('running')
    }
  });
});
