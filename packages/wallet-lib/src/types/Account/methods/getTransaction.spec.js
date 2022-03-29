const { expect } = require('chai');
const getTransaction = require('./getTransaction');
const mockAccountWithStorage = require("../../../test/mocks/mockAccountWithStorage");

let mockedAccount;
let fetchTransactionInfoCalledNb = 0;
describe('Account - getTransaction', function suite() {
  this.timeout(10000);
  before(() => {
    mockedAccount = mockAccountWithStorage({
      transport: {
        getTransaction: () => {
          fetchTransactionInfoCalledNb += 1;
          return null
        },
      }
    });
  });
  it('should correctly get a existing transaction', async () => {
    const tx = await getTransaction.call(mockedAccount, 'c8f0d780e6cedb9c37724f98cc3fecd6d5ad314db28e3cd439184bf25196ceb4');

    expect(tx.transaction.toObject()).to.deep.equal(expectedTx);

    expect(tx.metadata).to.deep.equal({
      blockHash: '000001810cf3b49ed94ae033f007923d3c243077f5f9e24b559b536087e8b960',
      height: 615751,
      isInstantLocked: null,
      isChainLocked: null
    });
  });

  it('should correctly try to fetch un unexisting transaction', async () => {
    expect(fetchTransactionInfoCalledNb).to.equal(0);
    const tx = await getTransaction.call(mockedAccount, '92151f239013c961db15bc91d904404d2ae0520929969b59b69b17493569d0d5');
    expect(fetchTransactionInfoCalledNb).to.equal(1);
    expect(tx).to.equal(null);
  });
});

const expectedTx = {
  hash: 'c8f0d780e6cedb9c37724f98cc3fecd6d5ad314db28e3cd439184bf25196ceb4',
  version: 3,
  inputs: [
    {
      prevTxId: 'a43f20bcb46fef22926745a27ca38f63c773f2c7c4cbb55aaf184b58b3755965',
      outputIndex: 0,
      sequenceNumber: 4294967295,
      script: '48304502210080185b616b2f8cb013e264b66146954cdc1597053ea8238467b896413b836039022071e71147dc45fc2dbde666c2774912c5bfd00b761268a3a4f8951375eb895c310121029671ae86f7eeb5c127568fddc977a1cce1f76ad64efa83c8ec9fcaef08ea9738',
      scriptString: '72 0x304502210080185b616b2f8cb013e264b66146954cdc1597053ea8238467b896413b836039022071e71147dc45fc2dbde666c2774912c5bfd00b761268a3a4f8951375eb895c3101 33 0x029671ae86f7eeb5c127568fddc977a1cce1f76ad64efa83c8ec9fcaef08ea9738'
    }
  ],
  outputs: [
    {
      satoshis: 10000,
      script: '76a9141ec5c66e9789c655ae068d35088b4073345fe0b088ac'
    },
    {
      satoshis: 224508306,
      script: '76a9147f6f4280f91e00126d927466cd48629439b763fa88ac'
    }
  ],
  nLockTime: 0
};

