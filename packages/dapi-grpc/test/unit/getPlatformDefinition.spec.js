const getPlatformDefinition = require('../../lib/getPlatformDefinition');

describe('getPlatformDefinition', () => {
  describe('v0', () => {
    it('should return loaded GRPC package definition', async () => {
      const platformDefinition = getPlatformDefinition(0);

      expect(platformDefinition).to.be.an('function');
      expect(platformDefinition).to.have.property('service');

      expect(platformDefinition.service).to.have.property('broadcastStateTransition');
      expect(platformDefinition.service.broadcastStateTransition.path).to.equal('/org.dash.platform.dapi.v0.Platform/broadcastStateTransition');

      expect(platformDefinition.service).to.have.property('getIdentity');
      expect(platformDefinition.service.getIdentity.path).to.equal('/org.dash.platform.dapi.v0.Platform/getIdentity');

      expect(platformDefinition.service).to.have.property('getDataContract');
      expect(platformDefinition.service.getDataContract.path).to.equal('/org.dash.platform.dapi.v0.Platform/getDataContract');

      expect(platformDefinition.service).to.have.property('getDocuments');
      expect(platformDefinition.service.getDocuments.path).to.equal('/org.dash.platform.dapi.v0.Platform/getDocuments');
    });
  });
});
