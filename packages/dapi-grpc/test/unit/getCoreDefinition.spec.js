const getCoreDefinition = require('../../lib/getCoreDefinition');

describe('getCoreDefinition', () => {
  it('should return loaded GRPC package definition', async () => {
    const coreDefinition = getCoreDefinition();

    expect(coreDefinition).to.be.an('function');
    expect(coreDefinition).to.have.property('service');

    expect(coreDefinition.service).to.have.property('getLastUserStateTransitionHash');
    expect(coreDefinition.service.getLastUserStateTransitionHash.path).to.equal('/org.dash.platform.dapi.v0.Core/getLastUserStateTransitionHash');

    expect(coreDefinition.service).to.have.property('subscribeToBlockHeadersWithChainLocks');
    expect(coreDefinition.service.subscribeToBlockHeadersWithChainLocks.path).to.equal('/org.dash.platform.dapi.v0.Core/subscribeToBlockHeadersWithChainLocks');

    expect(coreDefinition.service).to.have.property('updateState');
    expect(coreDefinition.service.updateState.path).to.equal('/org.dash.platform.dapi.v0.Core/updateState');
  });
});
