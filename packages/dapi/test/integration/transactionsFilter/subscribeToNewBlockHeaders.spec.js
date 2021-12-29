const sinon = require('sinon');

const subscribeToNewBlockHeaders = require('../../../lib/grpcServer/handlers/blockheaders-stream/subscribeToNewBlockHeaders');

describe('subscribeToNewTransactions', () => {
  it('should add blocks and latest chain lock signature in cache and send them back when historical data is sent', () => {
    const coreAPI = {};
    const mockZMQ = { on: sinon.stub(), topics: { hashblock: 'fake' } };
    const mediator = { on: sinon.stub(), once: sinon.stub() };

    subscribeToNewBlockHeaders(mediator, mockZMQ, coreAPI);

    mediator.emit('historicalDataSent');
  });
});
