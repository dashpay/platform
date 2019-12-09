const getCoreDefinition = require('../../lib/getCoreDefinition');

describe('getCoreDefinition', () => {
  it('should return loaded GRPC package definition', async () => {
    const coreDefinition = getCoreDefinition();

    expect(coreDefinition).to.be.an('function');
    expect(coreDefinition).to.have.property('service');

    expect(coreDefinition.service).to.have.property('sendTransaction');
    expect(coreDefinition.service.sendTransaction.path).to.equal('/org.dash.platform.dapi.v0.Core/sendTransaction');

    expect(coreDefinition.service).to.have.property('getTransaction');
    expect(coreDefinition.service.getTransaction.path).to.equal('/org.dash.platform.dapi.v0.Core/getTransaction');
  });
});
