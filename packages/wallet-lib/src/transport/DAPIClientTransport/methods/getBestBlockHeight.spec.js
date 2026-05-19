import { expect } from 'chai';

import DAPIClientTransport from '../DAPIClientTransport.js';

import getBlockchainStatus from '../../FixtureTransport/methods/getBlockchainStatus.js';

describe('transports - DAPIClientTransport - .getBestBlockHeight', function suite() {
  let fixture;
  let transport;
  let clientMock;

  beforeEach(() => {
    fixture = getBlockchainStatus();

    clientMock = {
      core: {
        getBestBlockHeight: () => 1,
      }
    }

    transport = new DAPIClientTransport(clientMock);
  })

  afterEach(() => {
    transport.disconnect();
  })

  it('should work', async () => {
    const res = await transport.getBestBlockHeight();

    expect(res).to.deep.equal(1);
  });
});
