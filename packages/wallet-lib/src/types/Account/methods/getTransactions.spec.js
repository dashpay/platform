const { expect } = require('chai');
const getTransactions = require('./getTransactions');

const getFixtureHDAccountWithStorage = require('../../../../fixtures/wallets/apart-trip-dignity/getFixtureAccountWithStorage');

const mockedHDSelf = {
  ...getFixtureHDAccountWithStorage(),
}
mockedHDSelf.getTransactions = getTransactions;

describe('Account - getTransactions', function suite() {
  this.timeout(10000);
  it('should get the transactions', () => {
    const transactions = getTransactions.call(mockedHDSelf);
    const transactionsHash = Object.keys(transactions);

    expect(transactionsHash).to.deep.equal([
      'a43845e580ad01f31bc06ce47ab39674e40316c4c6b765b6e54d6d35777ef456',
      'f230a9414bf577d93d6f7f2515d9b549ede78cfba4168920892970fa8aa1eef8',
      'd37b6c7dd449d605bea9997af8bbeed2f3fbbcb23a4068b1f1ad694db801912d',
      '7d1b78157f9f2238669f260d95af03aeefc99577ff0cddb91b3e518ee557a2fd',
      '1cbb35edc105918b956838570f122d6f3a1fba2b67467e643e901d09f5f8ac1b',
      'eb1a7fc8e3b43d3021653b1176f8f9b41e9667d05b65ee225d14c149a5b14f77',
      'c3fb3620ebd1c7678879b40df1495cc86a179b5a6f9e48ce0b687a5c6f5a1db5',
      '6f37b0d6284aab627c31c50e1c9d7cce39912dd4f2393f91734f794bc6408533',
      '6f76ca8038c6cb1b373bbbf80698afdc0d638e4a223be12a4feb5fd8e1801135',
      'e6b6f85a18d77974f376f05d6c96d0fdde990e733664248b1a00391565af6841'
    ]);
  });
});
