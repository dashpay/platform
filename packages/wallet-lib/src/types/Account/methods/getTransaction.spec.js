const { expect } = require('chai');
const mockedStore = require('../../../../fixtures/sirentonight-fullstore-snapshot-1562711703');
const getTransaction = require('./getTransaction');
const searchTransaction = require('../../Storage/methods/searchTransaction');

let mockedWallet;
let fetchTransactionInfoCalledNb = 0;
describe('Account - getTransaction', () => {
  before(() => {
    const storageHDW = {
      store: mockedStore,
      getStore: () => mockedStore,
      mappedAddress: {},
      searchTransaction,
      importTransactions: () => null,
    };
    const walletId = Object.keys(mockedStore.wallets)[0];
    mockedWallet = {
      walletId,
      index: 0,
      storage: storageHDW,
      transporter: {
        getTransaction: () => fetchTransactionInfoCalledNb += 1,
      },
    };
  });
  it('should correctly get a existing transaction', async () => {
    const tx = await getTransaction.call(mockedWallet, '92150f239013c961db15bc91d904404d2ae0520929969b59b69b17493569d0d5');
    expect(tx).to.deep.equal(expectedTx);
  });
  it('should correctly try to fetch un unexisting transaction', async () => {
    expect(fetchTransactionInfoCalledNb).to.equal(0);
    const tx = await getTransaction.call(mockedWallet, '92151f239013c961db15bc91d904404d2ae0520929969b59b69b17493569d0d5');
    expect(fetchTransactionInfoCalledNb).to.equal(1);
    expect(tx).to.equal(1);
  });
});

const expectedTx = {
  hash: '92150f239013c961db15bc91d904404d2ae0520929969b59b69b17493569d0d5',
  blockhash: '000000c5d6ca463ebbfddffe9a0a135312b6d8fc4eae2787b82b0fca9de7a554',
  blockheight: 29197,
  blocktime: 1562060795,
  fees: 522,
  size: 521,
  vout: [{
    value: '0.99990990',
    n: 0,
    scriptPubKey: {
      hex: '76a914ba84943e63925288d2972cd5d0c2e1e06873c7c688ac',
      asm: 'OP_DUP OP_HASH160 ba84943e63925288d2972cd5d0c2e1e06873c7c6 OP_EQUALVERIFY OP_CHECKSIG',
      addresses: ['ydKfMe2n4vWsrzvgfSieQsFFxM9XMoWBff'],
      type: 'pubkeyhash',
    },
    spentTxId: 'eabe39ada39b58d70c03e0e79b7d2c767ed1239dda436bbc5a58954285421acc',
    spentIndex: 1,
    spentHeight: 30969,
  }, {
    value: '1000.00000000',
    n: 1,
    scriptPubKey: {
      hex: '76a91485ada58442067249829d52ddd6c99c97a112749188ac',
      asm: 'OP_DUP OP_HASH160 85ada58442067249829d52ddd6c99c97a1127491 OP_EQUALVERIFY OP_CHECKSIG',
      addresses: ['yYWGjtb7XJqbXsUPfkaTWQKzcPYfmMp1Co'],
      type: 'pubkeyhash',
    },
    spentTxId: '5a5626c59f3830d5d9e7261bed5ced2694a343100c18d4a730b26639f5832944',
    spentIndex: 0,
    spentHeight: 29197,
  }],
  vin: [{
    hash: '0c25c534aeef8a151e8ce325882f80af647621b9f0a54f995f75c0d2994966ad',
    vout: 0,
    sequence: 4294967294,
    n: 0,
    scriptSig: {
      hex: '483045022100b24c95914f666ecb3ac41048110d7732b890b0d3fac9a9ff05560913e530430a022006660d72df91158f4d4710b75b9b05502a1332b5afca1ab5f98d012119f62553012103a6592040a30bf9254306a9d1086803cd450ae817ed5b4ba34e3e1b43d48bb783',
      asm: '3045022100b24c95914f666ecb3ac41048110d7732b890b0d3fac9a9ff05560913e530430a022006660d72df91158f4d4710b75b9b05502a1332b5afca1ab5f98d012119f62553[ALL] 03a6592040a30bf9254306a9d1086803cd450ae817ed5b4ba34e3e1b43d48bb783',
    },
    addr: 'yXzZsVfpPxjewfVd7oa2D6tBMHW7JbonBr',
    valueSat: 99991512,
    value: 0.99991512,
    doubleSpentTxID: null,
  }, {
    hash: '92056b727a3e37f5946dc18aa4f497ba9c0e3a328105e743175629bf7c8f3d37',
    vout: 0,
    sequence: 4294967294,
    n: 1,
    scriptSig: {
      hex: '483045022100fe69fdb70c0550b900960e9fbfd7254726a237c8b5688e5c9a7fba15947638fa02206ba0463b51922b56d0064c06b55c21328a39aa81312cf25ec982fa5c1eed9214012103353b4deb77923b026278d116e2007d6f97a058e42d35f1fd39efd5314705f844',
      asm: '3045022100fe69fdb70c0550b900960e9fbfd7254726a237c8b5688e5c9a7fba15947638fa02206ba0463b51922b56d0064c06b55c21328a39aa81312cf25ec982fa5c1eed9214[ALL] 03353b4deb77923b026278d116e2007d6f97a058e42d35f1fd39efd5314705f844',
    },
    addr: 'yhvXpqQjfN9S4j5mBKbxeGxiETJrrLETg5',
    valueSat: 50000000000,
    value: 500,
    doubleSpentTxID: null,
  }, {
    hash: 'be27a3dae2742aaca103fea0967edd9a6d0ef5cf90159af39f80ad5a7a50b7d6',
    vout: 0,
    sequence: 4294967294,
    n: 2,
    scriptSig: {
      hex: '47304402205d30afd97e5efbec984faae5be922a487d7adce1518a3214966198a5423c150d02204ea0e15a5fcf5b8034294c9d2b9d6fc638f75506fcac3530c91b960eaf2e6859012103353b4deb77923b026278d116e2007d6f97a058e42d35f1fd39efd5314705f844',
      asm: '304402205d30afd97e5efbec984faae5be922a487d7adce1518a3214966198a5423c150d02204ea0e15a5fcf5b8034294c9d2b9d6fc638f75506fcac3530c91b960eaf2e6859[ALL] 03353b4deb77923b026278d116e2007d6f97a058e42d35f1fd39efd5314705f844',
    },
    addr: 'yhvXpqQjfN9S4j5mBKbxeGxiETJrrLETg5',
    valueSat: 50000000000,
    value: 500,
    doubleSpentTxID: null,
  }],
  txlock: false,
  spendable: false,
};
