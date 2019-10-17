const getTransactionsFilterStreamDefinition = require('../../lib/getTransactionsFilterStreamDefinition');

describe('getTransactionsFilterStreamDefinition', () => {
  it('should return loaded transactions filter stream definition', async () => {
    const definition = getTransactionsFilterStreamDefinition();

    expect(definition).to.be.an('function');
    expect(definition).to.have.property('service');
    expect(definition.service).to.have.property('subscribeToTransactionsWithProofs');
    expect(definition.service.subscribeToTransactionsWithProofs.path).to.equal('/org.dash.platform.dapi.v0.TransactionsFilterStream/subscribeToTransactionsWithProofs');
  });
});
