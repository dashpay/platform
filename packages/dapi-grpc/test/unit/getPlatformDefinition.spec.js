const getPlatformDefinition = require('../../lib/getPlatformDefinition');

describe('getPlatformDefinition', () => {
  it('should return loaded GRPC package definition', async () => {
    const platformDefinition = getPlatformDefinition();

    expect(platformDefinition).to.be.an('function');
    expect(platformDefinition).to.have.property('service');

    expect(platformDefinition.service).to.have.property('applyStateTransition');
    expect(platformDefinition.service.applyStateTransition.path).to.equal('/org.dash.platform.dapi.v0.Platform/applyStateTransition');

    expect(platformDefinition.service).to.have.property('getIdentity');
    expect(platformDefinition.service.getIdentity.path).to.equal('/org.dash.platform.dapi.v0.Platform/getIdentity');

    expect(platformDefinition.service).to.have.property('getDataContract');
    expect(platformDefinition.service.getDataContract.path).to.equal('/org.dash.platform.dapi.v0.Platform/getDataContract');

    expect(platformDefinition.service).to.have.property('getDocuments');
    expect(platformDefinition.service.getDocuments.path).to.equal('/org.dash.platform.dapi.v0.Platform/getDocuments');
  });
});
