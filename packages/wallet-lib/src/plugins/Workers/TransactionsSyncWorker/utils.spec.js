const { expect } = require('chai');

const {
  PrivateKey, Transaction,
} = require('@dashevo/dashcore-lib');

const { mockUtxo } = require('../../../test/mocks/dashcore/transaction');

const utils = require('./utils');

describe('TransactionsSyncWorker - utils', () => {
  describe('#filterTransactionsForAddresses()', () => {
    const walletKeys = [
      new PrivateKey('livenet'),
      new PrivateKey('livenet'),
      new PrivateKey('livenet'),
    ];

    it('should filter wallet transactions', () => {
      const fromPK = walletKeys[0];
      const fromAddress = fromPK.toAddress();
      const changeAddress = walletKeys[1].toAddress();
      const transferAddress = walletKeys[2].toAddress();

      const toPK = new PrivateKey('livenet');
      const toAddress = toPK.toAddress();

      const utxo = mockUtxo({ address: fromAddress });
      const sentWithoutChange = new Transaction()
        .from(utxo)
        .to(toAddress, 1e8)
        .sign(fromPK);

      const sentWithChange = new Transaction()
        .from(utxo)
        .to(toAddress, 1e7)
        .change(changeAddress)
        .sign(fromPK);

      const otherWalletUtxo = mockUtxo({ address: toAddress });
      const received = new Transaction()
        .from(otherWalletUtxo)
        .to(fromAddress, 1e8)
        .sign(toPK);

      const accountTransfer = new Transaction()
        .from(utxo)
        .to(transferAddress, 1e8)
        .sign(fromPK);

      const unknown = new Transaction()
        .from(otherWalletUtxo)
        .to(new PrivateKey('livenet').toAddress(), 1e8)
        .sign(toPK);

      const result = utils.filterTransactionsForAddresses(
        [sentWithoutChange, sentWithChange, received, accountTransfer, unknown],
        walletKeys.map((key) => key.toAddress().toString()),
      );

      expect(result).to.deep.equal([sentWithoutChange, sentWithChange, received, accountTransfer]);
    });
  });
});
