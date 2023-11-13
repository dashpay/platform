import {getConfigMock} from "../../../../src/test/mock/getConfigMock.js";
import {getServicesScopeFactory} from "../../../../src/status/scopes/services.js";

describe('getServicesScopeFactory', () => {
  describe('#getServicesScope', () => {
    let mockDockerCompose;

    let config;
    let getServiceList;
    let getServicesScope;

    beforeEach(async function it() {
      mockDockerCompose = { inspectService: this.sinon.stub() };

      config = getConfigMock(this.sinon);

      getServiceList = this.sinon.stub();

      getServiceList.returns([{ name: 'mock', title: 'Mock service', image: 'fakeImageId' }]);

      getServicesScope = getServicesScopeFactory(mockDockerCompose, getServiceList);
    });

    it('should just work', async () => {
      mockDockerCompose.inspectService.resolves({
        Id: 'fakeId',
        State: {
          Status: 'running',
        },
        Config: {
          Image: 'fakeImageId',
        },
      });

      const scope = await getServicesScope(config);

      for (const [, service] of Object.entries(scope)) {
        expect(service.containerId).to.be.equal('fakeId');
        expect(service.image).to.be.equal('fakeImageId');
        expect(service.status).to.be.equal('running');
      }
    });

    it('should work if docker throws', async () => {
      mockDockerCompose.inspectService.throws();

      const scope = await getServicesScope(config);

      for (const [, service] of Object.entries(scope)) {
        expect(service.containerId).to.be.equal(null);
        expect(service.image).to.be.equal('fakeImageId');
        expect(service.status).to.be.equal(null);
      }
    });
  });
});
