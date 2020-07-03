const { expect } = require('chai');

const DAPIClientTransport = require('../DAPIClientTransport');

describe('transports - DAPIClientTransport - .getAddressSummary', function suite() {
  let fixture;
  let transport;
  let clientMock;

  beforeEach(() => {
    fixture = {
      addrStr: 'yYpSw2n2TRzoQaUShNsPo541z4bz4EJkGN',
      balance: 10,
      balanceSat: 1000000000,
      totalReceived: 10,
      totalReceivedSat: 1000000000,
      totalSent: 0,
      totalSentSat: 0,
      unconfirmedBalance: 0,
      unconfirmedBalanceSat: 0,
      unconfirmedTxApperances: 0,
      unconfirmedAppearances: 0,
      txApperances: 1,
      txAppearances: 1,
      transactions: [
        '3ab6ebc86b9cdea1580d376510e54a904f74fcaf38dfe9363fb44bcf33f83703',
      ],
    };

    clientMock = {
      core: {
        getAddressSummary: () => fixture,
      },
    }

    transport = new DAPIClientTransport(clientMock);
  })

  afterEach(() => {
    transport.disconnect();
  })

  it('should work', async () => {
    const res = await transport.getAddressSummary('yYpSw2n2TRzoQaUShNsPo541z4bz4EJkGN');

    expect(res).to.deep.equal(fixture);
  });
});
