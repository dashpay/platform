const {expect} = require('chai');
const {Transaction} = require('@dashevo/dashcore-lib');
const {each} = require('lodash');
const {WALLET_TYPES} = require('../CONSTANTS');

const categorizeTransactions = require('./categorizeTransactions');
const transactionsWithMetadataFixtures = require('../../fixtures/wallets/apart-trip-dignity/transactions-with-metadata.json');
const addressesFixtures = require('../../fixtures/wallets/apart-trip-dignity/addresses.json');
const walletFixtures = require('../../fixtures/wallets/apart-trip-dignity/wallet.json');
const expectedResults = require('../../fixtures/wallets/apart-trip-dignity/categorizeTransactions.expectedResults');

const prepareTransactionsWithMetadata = () => {
  const transactionsWithMetadata = [];
  each(transactionsWithMetadataFixtures, (transactionWithMetadataFixture) => {
    transactionsWithMetadata.push([new Transaction(transactionWithMetadataFixture[0]), transactionWithMetadataFixture[1]])
  });
  return transactionsWithMetadata;
};
const normalizeResults = (results) =>{
  return [...results].map((result)=>{
    result.transaction = result.transaction.toString()
    return result;
  })
}
/**
 * Fixtures data from real transactions on testnet.
 * Tx perform as follow (where [account, address] :
 *
 * TX 1 (1.8798) : Faucet -> [0,0](yTwEca67QSkZ6axGdpNFzWPaCj8zqYybY7-a43845e580ad01f31bc06ce47ab39674e40316c4c6b765b6e54d6d35777ef456)
 * TX 2 (0.1) : ExternalUser -> [0,1](yercyhdN9oEkZcB9BsW5ktFaDxFEuK6qXN-d37b6c7dd449d605bea9997af8bbeed2f3fbbcb23a4068b1f1ad694db801912d)
 * TX 3 (0.1) : ExternalUser -> [0,2](ygk3GCSba2J3L9G665Snozhj9HSkh5ByVE-7d1b78157f9f2238669f260d95af03aeefc99577ff0cddb91b3e518ee557a2fd)
 * TX 4 (8.4001) : Faucet -> [0,5](yMLhEsiP2ajSh8STmXnNmkWXtoHsmawZxd-eb1a7fc8e3b43d3021653b1176f8f9b41e9667d05b65ee225d14c149a5b14f77)
 * TX 5 (7.2921) : Faucet -> [0,4](ygHAVkMtYSqoTWHebDv7qkhMV6dHyuRsp2-1cbb35edc105918b956838570f122d6f3a1fba2b67467e643e901d09f5f8ac1b)
 * TX 6 (17.771) : [0,0][0,1][0,2][0,4][0,5] ->  [0,6](yj8rRKATAUHcAgXvNZekob58xKm2oNyvhv-f230a9414bf577d93d6f7f2515d9b549ede78cfba4168920892970fa8aa1eef8)
 * TX 7 (16.7) : [0,6] -> [0,6][0,7](yhaAB6e8m3F8zmGX7WAVYa6eEfmSrrnY8x-c3fb3620ebd1c7678879b40df1495cc86a179b5a6f9e48ce0b687a5c6f5a1db5)
 * TX 8 (12.6) : [0,6] -> [1,0](yYJmzWey5kNecAThet5BFxAga1F4b4DKQ2-6f37b0d6284aab627c31c50e1c9d7cce39912dd4f2393f91734f794bc6408533)
 * TX 9 (12) [1,0] -> [1,1](yNCqctyQaq51WU1hN5aNwsgMsZ5fRiB7GY-9cd3d44a87a7f99a33aebc6957105d5fb41698ef642189a36bac59ec0b5cd840)
 * TX 10 (11.5) [1,1] -> [0,8](yiXh4Yo5djG6QH8WzXkKm5EFzqLRJWakXz-6f76ca8038c6cb1b373bbbf80698afdc0d638e4a223be12a4feb5fd8e1801135)
 * TX 11 (10) [0,8] -> ExternalUser (yMX3ycrLVF2k6YxWQbMoYgs39aeTfY4wrB-e6b6f85a18d77974f376f05d6c96d0fdde990e733664248b1a00391565af6841)
 *
 * Therefore, we expect TransactionHistory to get us this exact order.
 * Fixture data are un-ordered as such [TXNo, FixtureElementNo].
 * [1,2][2,4][3,5][4,1][5,0][6,3][7,6][8,7][9,10][10,8][11,9]
 *
 */
describe('Utils - categorizeTransactions', function suite() {
  const transactionsWithMetadata = prepareTransactionsWithMetadata();

  const accountStore = {
    accounts: walletFixtures.store.accounts,
    network: walletFixtures.network,
    mnemonic: null,
    type: walletFixtures.type,
    identityIds: walletFixtures.identityIds,
    addresses: addressesFixtures
  };
  const accountIndex = 0;
  const walletType = WALLET_TYPES.HDWALLET;

  const set1 = [transactionsWithMetadata[2]];
  const expectedSetResult1 = [expectedResults[0]];
  const set2 = [transactionsWithMetadata[2], transactionsWithMetadata[4]];
  const expectedSetResult2 = [expectedResults[0], expectedResults[1]];
  const set3 = [transactionsWithMetadata[2], transactionsWithMetadata[4], transactionsWithMetadata[5]];
  const expectedSetResult3 = [expectedResults[0], expectedResults[1], expectedResults[2]];
  const set4 = [transactionsWithMetadata[2], transactionsWithMetadata[4], transactionsWithMetadata[5], transactionsWithMetadata[1]];
  const expectedSetResult4 = [expectedResults[0], expectedResults[1], expectedResults[2], expectedResults[3]];
  const set5 = [transactionsWithMetadata[2], transactionsWithMetadata[4], transactionsWithMetadata[5], transactionsWithMetadata[1], transactionsWithMetadata[0]];
  const expectedSetResult5 = [expectedResults[0], expectedResults[1], expectedResults[2], expectedResults[3], expectedResults[4]];
  const set6 = [transactionsWithMetadata[2], transactionsWithMetadata[4], transactionsWithMetadata[5], transactionsWithMetadata[1], transactionsWithMetadata[0], transactionsWithMetadata[3]];
  const expectedSetResult6 = [expectedResults[0], expectedResults[1], expectedResults[2], expectedResults[3], expectedResults[4], expectedResults[5]];
  const set7 = [transactionsWithMetadata[2], transactionsWithMetadata[4], transactionsWithMetadata[5], transactionsWithMetadata[1], transactionsWithMetadata[0], transactionsWithMetadata[3], transactionsWithMetadata[6]];
  const expectedSetResult7 = [expectedResults[0], expectedResults[1], expectedResults[2], expectedResults[3], expectedResults[4], expectedResults[5], expectedResults[6]];

  const set8 = [transactionsWithMetadata[2], transactionsWithMetadata[4], transactionsWithMetadata[5], transactionsWithMetadata[1], transactionsWithMetadata[0], transactionsWithMetadata[3], transactionsWithMetadata[6], transactionsWithMetadata[7]];
  const expectedSetResult8 = [expectedResults[0], expectedResults[1], expectedResults[2], expectedResults[3], expectedResults[4], expectedResults[5], expectedResults[6], expectedResults[7]];

  const set9 = [transactionsWithMetadata[2], transactionsWithMetadata[4], transactionsWithMetadata[5], transactionsWithMetadata[1], transactionsWithMetadata[0], transactionsWithMetadata[3], transactionsWithMetadata[6], transactionsWithMetadata[7], transactionsWithMetadata[10]];
  const expectedSetResult9 = [expectedResults[0], expectedResults[1], expectedResults[2], expectedResults[3], expectedResults[4], expectedResults[5], expectedResults[6], expectedResults[7], expectedResults[8]];
  const set10 = [transactionsWithMetadata[2], transactionsWithMetadata[4], transactionsWithMetadata[5], transactionsWithMetadata[1], transactionsWithMetadata[0], transactionsWithMetadata[3], transactionsWithMetadata[6], transactionsWithMetadata[7], transactionsWithMetadata[10], transactionsWithMetadata[8]];
  const expectedSetResult10 = [expectedResults[0], expectedResults[1], expectedResults[2], expectedResults[3], expectedResults[4], expectedResults[5], expectedResults[6], expectedResults[7], expectedResults[8], expectedResults[9]];
  const set11 = [transactionsWithMetadata[2], transactionsWithMetadata[4], transactionsWithMetadata[5], transactionsWithMetadata[1], transactionsWithMetadata[0], transactionsWithMetadata[3], transactionsWithMetadata[6], transactionsWithMetadata[7],transactionsWithMetadata[10], transactionsWithMetadata[8], transactionsWithMetadata[9]];
  const expectedSetResult11 = [expectedResults[0], expectedResults[1], expectedResults[2], expectedResults[3], expectedResults[4], expectedResults[5], expectedResults[6], expectedResults[7], expectedResults[8], expectedResults[9],expectedResults[10]];



  it('should correctly categorize transaction', function () {
    const result1 = categorizeTransactions(set1, accountStore, accountIndex, walletType);
    expect(normalizeResults(result1)).to.deep.equal(expectedSetResult1)

    const result2 = categorizeTransactions(set2, accountStore, accountIndex, walletType);
    expect(normalizeResults(result2)).to.deep.equal(expectedSetResult2)

    const result3 = categorizeTransactions(set3, accountStore, accountIndex, walletType);
    expect(normalizeResults(result3)).to.deep.equal(expectedSetResult3)

    const result4 = categorizeTransactions(set4, accountStore, accountIndex, walletType);
    expect(normalizeResults(result4)).to.deep.equal(expectedSetResult4)

    const result5 = categorizeTransactions(set5, accountStore, accountIndex, walletType);
    expect(normalizeResults(result5)).to.deep.equal(expectedSetResult5)

    const result6 = categorizeTransactions(set6, accountStore, accountIndex, walletType);
    expect(normalizeResults(result6)).to.deep.equal(expectedSetResult6)

    const result7 = categorizeTransactions(set7, accountStore, accountIndex, walletType);
    expect(normalizeResults(result7)).to.deep.equal(expectedSetResult7)

    const result8 = categorizeTransactions(set8, accountStore, accountIndex, walletType);
    expect(normalizeResults(result8)).to.deep.equal(expectedSetResult8)

    const result9 = categorizeTransactions(set9, accountStore, accountIndex, walletType);
    expect(normalizeResults(result9)).to.deep.equal(expectedSetResult9)

    const result10 = categorizeTransactions(set10, accountStore, accountIndex, walletType);
    expect(normalizeResults(result10)).to.deep.equal(expectedSetResult10)

    const result11 = categorizeTransactions(set11, accountStore, accountIndex, walletType);
    expect(normalizeResults(result11)).to.deep.equal(expectedSetResult11)
  });
});
