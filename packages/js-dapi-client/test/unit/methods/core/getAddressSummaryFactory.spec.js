const getAddressSummaryFactory = require('../../../../lib/methods/core/getAddressSummaryFactory');

describe('getAddressSummaryFactory', () => {
  let getAddressSummary;
  let jsonRpcTransport;

  beforeEach(function beforeEach() {
    jsonRpcTransport = {
      request: this.sinon.stub(),
    };
    getAddressSummary = getAddressSummaryFactory(jsonRpcTransport);
  });

  it('should return a summary for an address', async () => {
    const summary = {
      addrStr: 'yXdxAYfK8eJgQmHpUzMaKEBhqwKQWKSezS',
      balance: 4173964.74940914,
      balanceSat: 417396474940914,
      totalReceived: 4287576.24940914,
      totalReceivedSat: 428757624940914,
      totalSent: 113611.5,
      totalSentSat: 11361150000000,
      unconfirmedBalance: 0,
      unconfirmedBalanceSat: 0,
      unconfirmedTxApperances: 0,
      txApperances: 27434,
      transactions: ['4f46066bd50cc2684484407696b7949e82bd906ea92c040f59a97cba47ed8176', '8890a0ee95a17f6723ab2d9a0bdd579351b9220738ad34f5b49cbe63f09b082a'],
    };

    jsonRpcTransport.request.resolves(summary);

    const address = 'yTMDce5yEpiPqmgPrPmTj7yAmQPJERUSVy';
    const options = {};

    const result = await getAddressSummary(address, options);

    expect(result).to.be.an('object');
    expect(result).to.deep.equal(summary);
    expect(jsonRpcTransport.request).to.be.calledOnceWithExactly(
      'getAddressSummary',
      {
        address,
        noTxList: options.noTxList,
        from: options.from,
        to: options.to,
        fromHeight: options.fromHeight,
        toHeight: options.toHeight,
      },
      options,
    );
  });
});
