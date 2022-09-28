const { expect } = require('chai');
const getTransaction = require('./getTransaction');
const getFixtureHDAccountWithStorage = require("../../../../fixtures/wallets/apart-trip-dignity/getFixtureAccountWithStorage");

let mockedAccount;
let fetchTransactionInfoCalledNb = 0;
describe('Account - getTransaction', function suite() {
  this.timeout(10000);
  before(() => {
    mockedAccount = getFixtureHDAccountWithStorage();

    mockedAccount.transport = {
      getTransaction: () => {
        fetchTransactionInfoCalledNb += 1;
        return null
      },
    }
  });
  it('should correctly get a existing transaction', async () => {
    const tx = await getTransaction.call(mockedAccount, 'a43845e580ad01f31bc06ce47ab39674e40316c4c6b765b6e54d6d35777ef456');

    expect(tx.transaction.toObject()).to.deep.equal(expectedTx);

    expect(tx.metadata).to.deep.equal({
      "blockHash": "000001deee9f99e8219a9abcaaea135dbaae8a9b0f1ea214e6b6a37a5c5b115d",
      "height": 555506,
      "isInstantLocked": true,
      "isChainLocked": true
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
  "hash": "a43845e580ad01f31bc06ce47ab39674e40316c4c6b765b6e54d6d35777ef456",
  "version": 2,
  "inputs": [
    {
      "prevTxId": "11802a0d6221636a93023f73750946ace488a79d3074ba93abb4edc19bf91efd",
      "outputIndex": 0,
      "sequenceNumber": 4294967294,
      "script": "483045022100dfb220a840d597179abdf49692ad64c1c0da785041975b00aee03c9625639cf202204d06eade5cca19fab1e10b1d6e1b67c77626a0e88bb4d5f61bd57293b4b64217012102295ecb812ccf52deaf304bebfe3a59a644f05bac81241ea1e3a2f8750064cbf6",
      "scriptString": "72 0x3045022100dfb220a840d597179abdf49692ad64c1c0da785041975b00aee03c9625639cf202204d06eade5cca19fab1e10b1d6e1b67c77626a0e88bb4d5f61bd57293b4b6421701 33 0x02295ecb812ccf52deaf304bebfe3a59a644f05bac81241ea1e3a2f8750064cbf6"
    },
    {
      "prevTxId": "19953851c7a425045d3b6b4f56b7d5116fc1648444e5c37eba29ea65ee264269",
      "outputIndex": 0,
      "sequenceNumber": 4294967294,
      "script": "483045022100beff3263b7c99720e99af9ec146c818701efb0130603f1570f427b74aef8521802202e660bb9f7ea156f91addd5fe47cbd2c2bf388cc6e1eff3a39adffd89d26d346012102c33942799f7cbf4a7d12f1b3e52cb80cc4de083b997d3e63915df9973d5bce2a",
      "scriptString": "72 0x3045022100beff3263b7c99720e99af9ec146c818701efb0130603f1570f427b74aef8521802202e660bb9f7ea156f91addd5fe47cbd2c2bf388cc6e1eff3a39adffd89d26d34601 33 0x02c33942799f7cbf4a7d12f1b3e52cb80cc4de083b997d3e63915df9973d5bce2a"
    },
    {
      "prevTxId": "2dc8e2adfb30902269fa77dbf0de94f1f04ab3e8b1dbe1dd074a39a864993e96",
      "outputIndex": 0,
      "sequenceNumber": 4294967294,
      "script": "483045022100ff67776932e7a32520aa131f76bdfd6737650ad3b11edbdf466cca83f691b0e60220633bcbedebacffd53ceb7e9cdbd47928d7c2849f49ac1f8efb9f384c1a4ee46301210371c0bc42e08de059a8829730abb16f3d40cff87e5ad85d65c4a0a949d9c4b524",
      "scriptString": "72 0x3045022100ff67776932e7a32520aa131f76bdfd6737650ad3b11edbdf466cca83f691b0e60220633bcbedebacffd53ceb7e9cdbd47928d7c2849f49ac1f8efb9f384c1a4ee46301 33 0x0371c0bc42e08de059a8829730abb16f3d40cff87e5ad85d65c4a0a949d9c4b524"
    },
    {
      "prevTxId": "40cf2327c923487ce9789c58a1273ddd9bb87a8d30975dc298335c125065e11f",
      "outputIndex": 0,
      "sequenceNumber": 4294967294,
      "script": "483045022100cca348c7ab16fac28b3bba502be54a9e3766b7da9821a90605f370b75840569702207d082510aa493988e09da046355b018781718208f8a954e14ea33d608ae59625012103699b9402e109ed9d0c67c6a45be5cf5f1236c44bb9fc4b07a2f3392ba0b64172",
      "scriptString": "72 0x3045022100cca348c7ab16fac28b3bba502be54a9e3766b7da9821a90605f370b75840569702207d082510aa493988e09da046355b018781718208f8a954e14ea33d608ae5962501 33 0x03699b9402e109ed9d0c67c6a45be5cf5f1236c44bb9fc4b07a2f3392ba0b64172"
    },
    {
      "prevTxId": "4bb38b9207953d4658c64e6ad986eab05a42e50c72bd0f3bf07d7dd8b31f25ce",
      "outputIndex": 0,
      "sequenceNumber": 4294967294,
      "script": "47304402203ae564ff74b08b1f96bf857f51448434418d747a02039ec1ee109a4f5d8e8106022072f8769bd175416d22f44011f7e67aec301f08573c9937be7e4a09c394c7396601210311bae874933a4503a61d1c8c2e5b57b1a278d28d4892af4bd79ab8a731495265",
      "scriptString": "71 0x304402203ae564ff74b08b1f96bf857f51448434418d747a02039ec1ee109a4f5d8e8106022072f8769bd175416d22f44011f7e67aec301f08573c9937be7e4a09c394c7396601 33 0x0311bae874933a4503a61d1c8c2e5b57b1a278d28d4892af4bd79ab8a731495265"
    },
    {
      "prevTxId": "b21e8513b29a43b3169b857c466cc626859d76e374fc5dc7771f4a0df8fe2daf",
      "outputIndex": 0,
      "sequenceNumber": 4294967294,
      "script": "47304402200b49b7059064efb57df453dc2d20002f09b5266bc825760ef81624771f13920802200782616b8c4fb7b5eff94fdf865e6ddc4530d3932b97fdc3a747e8c451f0314c012103a94131f28f8efd67f47f2496ff6e8d9069a3a7df97202a33e90e16f257d03729",
      "scriptString": "71 0x304402200b49b7059064efb57df453dc2d20002f09b5266bc825760ef81624771f13920802200782616b8c4fb7b5eff94fdf865e6ddc4530d3932b97fdc3a747e8c451f0314c01 33 0x03a94131f28f8efd67f47f2496ff6e8d9069a3a7df97202a33e90e16f257d03729"
    },
    {
      "prevTxId": "d6fd2b6ea7d186a2211076188594cacb61df415051876fa198ca4c2205ef4f34",
      "outputIndex": 0,
      "sequenceNumber": 4294967294,
      "script": "47304402202a24d1123775641269c6f748d3e4dad08a682e4e334a9b73c7df84f6c22e8e7d022022c0cc2225d3f14cb58a6fb3e4bf23c0f33252d9040a9ca9ef66eb17742a476f01210347301de4c9ba7f46b0f27cb82ae70a73749821e2951d3c87c2f0d56648635d1c",
      "scriptString": "71 0x304402202a24d1123775641269c6f748d3e4dad08a682e4e334a9b73c7df84f6c22e8e7d022022c0cc2225d3f14cb58a6fb3e4bf23c0f33252d9040a9ca9ef66eb17742a476f01 33 0x0347301de4c9ba7f46b0f27cb82ae70a73749821e2951d3c87c2f0d56648635d1c"
    }
  ],
  "outputs": [
    {
      "satoshis": 1823313,
      "script": "76a91440ca54360086cc0fbd69d862db58ab2b6d22805888ac"
    },
    {
      "satoshis": 187980000,
      "script": "76a914538da44e7136cc994023d89a7b4b3d02ac0e573988ac"
    }
  ],
  "nLockTime": 555505
}

