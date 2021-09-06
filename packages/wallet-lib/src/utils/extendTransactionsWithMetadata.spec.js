const { expect } = require('chai');
const { Transaction } = require('@dashevo/dashcore-lib');
const extendTransactionsWithMetadata = require('./extendTransactionsWithMetadata');

const rawtx = '03000000012d9101b84d69adf1b168403ab2bcfbf3d2eebbf87a99a9be05d649d47d6c7bd3010000006a47304402201cc3d6887d5161eba36a5e6fb1ccd8e8f9eeda7fe95b4fb0a1accb99eeba0223022040d0df81fde8f59c807e541ca5bcfc9d7450f76657aeb44c708fa7d65b7d58410121038cdae47fceb5b117cd3ef5bdf8c9f2a83679a9105d012095762067bdb2351ceaffffffff0280969800000000001976a914e00939d2ec2f885f5e7dc7b9f5b06dcf868d0c4b88acabfe261f000000001976a914f03286cbb7954ea6affa9654af6cfe1210dd0c6288ac00000000';
const tx = new Transaction(rawtx);
const txid = '7d1b78157f9f2238669f260d95af03aeefc99577ff0cddb91b3e518ee557a2fd';
const transactions = {};
transactions[txid] = tx;

const transactionsMetadata = {};
transactionsMetadata[txid] = {
  blockHash: '0000012cf6377c6cf2b317a4deed46573c09f04f6880dca731cc9ccea6691e19',
      height: 555508,
      instantLocked: true,
      chainLocked: true
};

describe('Utils - extendTransactionWithMetadata', function suite() {
  it('should correctly extend metadata from transaction', function () {
    const result = extendTransactionsWithMetadata(transactions, transactionsMetadata);
    const expectedResults = [[tx,{ 
    blockHash: '0000012cf6377c6cf2b317a4deed46573c09f04f6880dca731cc9ccea6691e19',
    height: 555508,
    instantLocked: true,
    chainLocked: true
   }]];
    expect(result).to.deep.equal(expectedResults);
  });
});
