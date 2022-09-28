const { Transaction } = require('@dashevo/dashcore-lib');
const { expect } = require('chai');
const { WALLET_TYPES } = require('../CONSTANTS');
const filterTransactions = require('./filterTransactions');

const fixtureAddressesStore = {
  external: {},
  internal: {},
  misc: {}
};
const externalAddresses = [
  'yTwEca67QSkZ6axGdpNFzWPaCj8zqYybY7',
  'yercyhdN9oEkZcB9BsW5ktFaDxFEuK6qXN',
  'ygk3GCSba2J3L9G665Snozhj9HSkh5ByVE',
  'ybuL6rM6dgrKzCg8s99f3jxGuv5oz5JcDA',
  'ygHAVkMtYSqoTWHebDv7qkhMV6dHyuRsp2',
  'yMLhEsiP2ajSh8STmXnNmkWXtoHsmawZxd',
  'yj8rRKATAUHcAgXvNZekob58xKm2oNyvhv',
  'yhaAB6e8m3F8zmGX7WAVYa6eEfmSrrnY8x',
  'yiXh4Yo5djG6QH8WzXkKm5EFzqLRJWakXz',
  'yQYv3Um6DsdtANo1ZPTUte75wAGMstLRex',
  'yiYPJmu7eEm1cXUNumQRdjv1fvPhsfgMS4',
  'yii4aUZhNfL6EWN9KAgAFrJzGJmqHnF4wx',
  'yLpTquSct2SGz2Ka45uTPDd81Kzro2Jt2k',
  'yMiJtpzb1Qthy9TGnavsf5NZ6EZZa4j9q3',
  'yacgSfW7RkwWakEZPg8USAVdzCypiG3vxS',
  'yVvrmoRPFLy6nUpCQBT8ZExxF5wF3DhiGU',
  'yaJf2aG6cFUtfv4o6TuEKsh5kr4xq5iAY4',
  'yfardJQ4ucgWLKQPaRHGMRMbSGm5H4ExJR',
  'yLSCqx7dcM5JKR2fG7vHbF2axMvuYqomaw',
  'yVij8XpJ78LM5hepSV1KF7T8vRpUEXCpK5',
  'ydJpjuJGossAZR7S5oS7cWvjygEwoj8Xwp',
  'yW3TmWnmhvpxRbgFcQ8oXqDRkn3RhRH6jj',
  'yRegVX85DThKRkH8C61TtRacfzrkiBfNy5',
  'yPtDCqDFRe1JuDp8pvdiEMQMz2erGwS3VG',
  'yM9pSw3L4oBfG7uQL5o522Hu3WTvy9awgZ',
  'yNC6qYJYungzuk5XUynDFKCn54Dy8ngox4',
];
const internalAddresses = [
  'yNDpPsJqXKM36zHSNEW7c1zSvNnrZ699FY',
  'yLk4Hw3w4zDudrDVP6W8J9TggkY57zQUki',
  'yirJaK8KCE5YAmwvLadizqFw3TCXqBuZXL',
  'yhdRfg5gNr587dtEC4YYMcSHmLVEGqqtHc',
  'yYwKP1FQae5kbjXkmuirGx6Xzf8NzHpLqW',
  'yX9gmsm8aSxZZjYhq4w35aidT7qbhcpNjU',
  'ybgXCTGMHEBbQeUib8c3xAjtGAc12XtWiU',
  'yS31WpdMT2b34uL9C37fbUoACHhiupHCyP',
  'yTSpFqRoX3vyN286AUtKKhgmX5Xb41YKQe',
  'yQU5YsqN7psTTASuYbcMi7N5nNZGaxXb2X',
  'yVGGFj9BLgEab5rucSGLC6UGVLQKB4U1wJ',
  'yQCh5yYCHEbJzgSJE9rdHiqXHidKm3kwr5',
  'yX7T3Ac3yaLk5CTC5UaR93Fc7SjYkeT5hn',
  'yXx3WXq8kYNPbYEg5U6bL8Xfih4g5LCYVo',
  'yYnLMTz3jCi2KKKNuo3TVkEAGyUFg8tgkJ',
  'yiKa1dA6B4tSTNJqJP9Y5pQfQEffnQQDTL',
  'yf7vcuDnE9DVhXdMfBMQQTEi43otYQzkWE',
  'yTmSmocwERCeRHqNNG5SbpYKUra1HTmj8m',
  'yivUe5NeJsGsREwPQZUGYaTSwWB3E1oLcz',
  'ygfsZojdfW9UjCRU4ra95Aq6YgCC7UqZFx',
  'yU9fdXaUVtefwDZvxjJAr9xj1z2MtYi34A',
  'yXgMN6FgrgZCnTN1vhoZMh8afKMBmi3JC4',
  'yiqaCbXscvR8y3VFYMzdaKCaAGuDuZxMzt',
  'ydcgWDxheSxrLAqDBP4JXBndMCzUNf77gq',
  'yYccLAwvYUDkjSp8VXvEyZ1t2i799pGrde',
  'yMRfbbqFZvojgYZCshdJNWJHruQb3DuCSC'
];

const fixtureTransactions = {};

const mockTransactions = (amount) => {
  return Array.from({ length: amount }).map((_, index) => {
    const tx = new Transaction();

    // Produce random lock date
    const date = new Date()
    date.setMinutes(Math.floor(Math.random() * 60) + index)

    tx.lockUntilDate(date)

    return tx;
  })
}

for(let i = 0; i<=externalAddresses.length; i++){
  let path = `m/44'/1'/0'/0/${i}`;

  let externalTransactions = [];

  // Leave some addresses without any tx
  if (i < externalAddresses.length / 2) {
    externalTransactions = mockTransactions(3);
  }

  fixtureAddressesStore.external[path] = {
    path,
    index: i,
    transactions: externalTransactions.map(tx => tx.hash),
    balanceSat: 0,
    unconfirmedBalanceSat: 0,
    utxos: {},
    address: externalAddresses[i]
  }

  path = `m/44'/1'/0'/1/${i}`;

  let internalTransactions = [];
  // Leave some addresses without any tx
  if (i < internalAddresses.length / 2) {
    internalTransactions = mockTransactions(3);

    if (externalTransactions.length) {
      // Simulate TX change return from the external TX
      internalTransactions.push(externalTransactions[0])
    }
  }

  fixtureAddressesStore.internal[path] = {
    path,
    index: i,
    transactions: internalTransactions.map(tx => tx.hash),
    balanceSat: 0,
    unconfirmedBalanceSat: 0,
    utxos: {},
    address: internalAddresses[i]
  };

  [...externalTransactions, ...internalTransactions].forEach(tx => {
    Object.assign(fixtureTransactions, { [tx.hash]: tx})
  })
}

describe('Utils - filterTransactions', function suite() {
  it('should correctly filter a transaction', () => {
    const accountStore = {
      addresses: fixtureAddressesStore,
    };
    const walletType = WALLET_TYPES.HDWALLET;
    const accountIndex = 0;
    const result = filterTransactions(accountStore, walletType, accountIndex, fixtureTransactions);
    result.sort((a,b) => a.nLockTime - b.nLockTime);
    const expectedResult = Object.values(fixtureTransactions)
      .sort((a, b) => a.nLockTime - b.nLockTime)

    expect(result).to.deep.equal(expectedResult);

    const accountIndex1 = 1;
    const result2 = filterTransactions(accountStore, walletType, accountIndex1, fixtureTransactions);
    const expectedEmptyResult = [];
    expect(result2).to.deep.equal(expectedEmptyResult);
  });
});
