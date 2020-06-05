const getUTXOFactory = require('../../../../lib/methods/core/getUTXOFactory');

describe('getUTXOFactory', () => {
  let getUTXO;
  let jsonRpcTransport;
  let options;
  let utxo;

  beforeEach(function beforeEach() {
    utxo = [{
      address: 'ygPcCwVy7Fxg7ruxZzqVYdPLtvw7auHAFh',
      txid: '42c56c8ec8e2cdf97a56bf6290c43811ff59181a184b70ebbe3cb66b2970ced2',
      vout: 1,
      scriptPubKey: '76a914dc2bfda564dc6217c55c842d65cc0242e095d2d788ac',
      amount: 999.81530001,
      satoshis: 99981530001,
      confirmations: 0,
      ts: 1529847344,
    }];

    options = {
      from: 10,
      to: 10000,
    };

    jsonRpcTransport = {
      request: this.sinon.stub().resolves(utxo),
    };
    getUTXO = getUTXOFactory(jsonRpcTransport);
  });

  it('should return UTXO', async () => {
    const address = 'yXdxAYfK8eJgQmHpUzMaKEBhqwKQWKSezS';

    const result = await getUTXO(address, options);
    expect(result).to.equal(utxo);
    expect(jsonRpcTransport.request).to.be.calledOnceWithExactly(
      'getUTXO',
      {
        address,
        from: options.from,
        to: options.to,
        fromHeight: options.fromHeight,
        toHeight: options.toHeight,
      },
      options,
    );
  });
});
