const { Transaction } = require('@dashevo/dashcore-lib');
const { expect } = require('chai');
const { WALLET_TYPES } = require('../CONSTANTS');
const filterTransactions = require('./filterTransactions');

const rawtx = '03000000012d9101b84d69adf1b168403ab2bcfbf3d2eebbf87a99a9be05d649d47d6c7bd3010000006a47304402201cc3d6887d5161eba36a5e6fb1ccd8e8f9eeda7fe95b4fb0a1accb99eeba0223022040d0df81fde8f59c807e541ca5bcfc9d7450f76657aeb44c708fa7d65b7d58410121038cdae47fceb5b117cd3ef5bdf8c9f2a83679a9105d012095762067bdb2351ceaffffffff0280969800000000001976a914e00939d2ec2f885f5e7dc7b9f5b06dcf868d0c4b88acabfe261f000000001976a914f03286cbb7954ea6affa9654af6cfe1210dd0c6288ac00000000';
const tx = new Transaction(rawtx);
const txid = '7d1b78157f9f2238669f260d95af03aeefc99577ff0cddb91b3e518ee557a2fd';
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
for(let i = 0; i<=externalAddresses.length; i++){
  fixtureAddressesStore.external[i] = {
    path: `m/44'/1'/0'/0/${i}`,
    index: i,
    transactions: [],
    balanceSat: 0,
    unconfirmedBalanceSat: 0,
    utxos: {},
    address: externalAddresses[i]
  }
  fixtureAddressesStore.internal[i] = {
    path: `m/44'/1'/0'/1/${i}`,
    index: i,
    transactions: [],
    balanceSat: 0,
    unconfirmedBalanceSat: 0,
    utxos: {},
    address: internalAddresses[i]
  };
}
const fixtureTransactions = {};
fixtureTransactions[txid] = tx;
fixtureAddressesStore.external[2].transactions.push(txid);
describe('Utils - filterTransactions', function suite() {
  it('should correctly filter a transaction', () => {
    const accountStore = {
      addresses: fixtureAddressesStore,
    };
    const walletType = WALLET_TYPES.HDWALLET;
    const accountIndex = 0;
    const result = filterTransactions(accountStore, walletType, accountIndex, fixtureTransactions);
    const expectedResult = [tx];
    expect(result).to.deep.equal(expectedResult);

    const accountIndex1 = 1;
    const result2 = filterTransactions(accountStore, walletType, accountIndex1, fixtureTransactions);
    const expectedEmptyResult = [];
    expect(result2).to.deep.equal(expectedEmptyResult);
  });
});
