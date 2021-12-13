const { expect } = require('chai');
const rehydrateState = require('./rehydrateState');
const { Transaction, BlockHeader, InstantLock } = require("@dashevo/dashcore-lib");

const storeMock = {
  "transactions": {
    "d37eb9f1b41c45134926d418486f2d8f57778cb949f4069a0b4c91955934913b": {
      "hash": "d37eb9f1b41c45134926d418486f2d8f57778cb949f4069a0b4c91955934913b",
      "version": 2,
      "inputs": [
        {
          "prevTxId": "ba657cdff1bdf2010e2b394f7db81aa429a94574c7fdcc5da4a8c5be751bd419",
          "outputIndex": 1,
          "sequenceNumber": 4294967294,
          "script": "483045022100eb7f103a5755e79a25179f289b74033d13e7b5a252a4911bba1c908d0db4361002203a664917ebbb4f5c1d81409898c05f807e23dd324d4458f32a6fc5b42ad8e3830121024b181f43670ebc67826e07885170029e01fd830bfcfa43862d694224d8c7e162",
          "scriptString": "72 0x3045022100eb7f103a5755e79a25179f289b74033d13e7b5a252a4911bba1c908d0db4361002203a664917ebbb4f5c1d81409898c05f807e23dd324d4458f32a6fc5b42ad8e38301 33 0x024b181f43670ebc67826e07885170029e01fd830bfcfa43862d694224d8c7e162"
        }
      ],
      "outputs": [
        {
          "satoshis": 82716538,
          "script": "76a9142f7d620394f5604854e495e6a78773287651faab88ac"
        },
        {
          "satoshis": 105580000,
          "script": "76a91428a46d7ae4d50ec3a5dbb847b01a60818905e98e88ac"
        }
      ],
      "nLockTime": 627066
    },
    "26faad59f6f3aef120323fb3aa9b0a3aaccf56ae770ff83099e3e86324d3a0da": {
      "hash": "26faad59f6f3aef120323fb3aa9b0a3aaccf56ae770ff83099e3e86324d3a0da",
      "version": 3,
      "inputs": [
        {
          "prevTxId": "d37eb9f1b41c45134926d418486f2d8f57778cb949f4069a0b4c91955934913b",
          "outputIndex": 1,
          "sequenceNumber": 4294967295,
          "script": "4730440220645a3cfe6798b2d4e5943209c2f241725c0e809c82bd3985743854bbebfeca48022036fd8bf98960d4d1deea37e5cc5833cb64178bc2e27a1628f83250e9e2ec69290121024c61a9a159104ad5e027754b19514347c5ee4a7f5ede8cc379f5f987f2fdb0b0",
          "scriptString": "71 0x30440220645a3cfe6798b2d4e5943209c2f241725c0e809c82bd3985743854bbebfeca48022036fd8bf98960d4d1deea37e5cc5833cb64178bc2e27a1628f83250e9e2ec692901 33 0x024c61a9a159104ad5e027754b19514347c5ee4a7f5ede8cc379f5f987f2fdb0b0"
        }
      ],
      "outputs": [
        {
          "satoshis": 10000,
          "script": "76a9141ec5c66e9789c655ae068d35088b4073345fe0b088ac"
        },
        {
          "satoshis": 105569753,
          "script": "76a91499c256c6f6724b926b7d903b10547bf712faeebd88ac"
        }
      ],
      "nLockTime": 0
    }
  },
  "wallets": {
    "799ea77395": {
      "accounts": {
        "m/44'/1'/0'": {
          "label": null,
          "path": "m/44'/1'/0'",
          "network": "testnet",
          "blockHeight": 627626,
          "blockHash": "00000053b6c3b0b56f64607ad77524922fbb4560306fa31f165ae36e11551d77"
        }
      },
      "network": "testnet",
      "mnemonic": null,
      "type": null,
      "identityIds": [],
      "addresses": {
        "external": {
          "m/44'/1'/0'/0/0": {
            "path": "m/44'/1'/0'/0/0",
            "index": 0,
            "address": "yQ2LrWYLRdUdDnofwqscghwSgCghd2BMPv",
            "transactions": [
              "d37eb9f1b41c45134926d418486f2d8f57778cb949f4069a0b4c91955934913b",
              "26faad59f6f3aef120323fb3aa9b0a3aaccf56ae770ff83099e3e86324d3a0da"
            ],
            "balanceSat": 0,
            "unconfirmedBalanceSat": 0,
            "utxos": {
              "adfc0ad1dd803fc1541f87711e794936c3fd99b763d0b26e77dd463425382833-1": {
                "address": "yQ2LrWYLRdUdDnofwqscghwSgCghd2BMPv",
                "txid": "26faad59f6f3aef120323fb3aa9b0a3aaccf56ae770ff83099e3e86324d3a0da",
                "vout": 1,
                "scriptPubKey": "76a91421b4db2de99d02a16092fa0aa0f9bb32d87353a588ac",
                "amount": 0.82706400
              }
            },
            "fetchedLast": 0,
            "used": true
          }
        },
        "internal": {
          "m/44'/1'/0'/1/0": {
            "path": "m/44'/1'/0'/1/0",
            "index": 0,
            "address": "yaLT3TjS7jdDVgAjDBCRSm1rHbsMeLXAx4",
            "transactions": [
              "adfc0ad1dd803fc1541f87711e794936c3fd99b763d0b26e77dd463425382833"
            ],
            "balanceSat": 0,
            "unconfirmedBalanceSat": 0,
            "utxos": {
              "adfc0ad1dd803fc1541f87711e794936c3fd99b763d0b26e77dd463425382833-1": {
                "address": "yaLT3TjS7jdDVgAjDBCRSm1rHbsMeLXAx4",
                "txid": "26faad59f6f3aef120323fb3aa9b0a3aaccf56ae770ff83099e3e86324d3a0da",
                "vout": 1,
                "scriptPubKey": "76a91421b4db2de99d02a16092fa0aa0f9bb32d87353a588ac",
                "amount": 0.0001
              }
            },
            "fetchedLast": 0,
            "used": true
          }
        },
        "misc": {}
      }
    }
  },
  "chains": {
    "testnet": {
      "name": "testnet",
      "blockHeaders": {
        "000000059885815cfc06ba74b814200d29658394dbe5d1e93948a8587947747b": {
          "hash": "000000059885815cfc06ba74b814200d29658394dbe5d1e93948a8587947747b",
          "version": 536870912,
          "prevHash": "000000c520efd2047f0b6f0c1c75e0382f8a9b7d76bb140bde3ada10c62e8b0d",
          "merkleRoot": "ef292bfb7965402e57dfeb4ee8bad0055c216c4c5a4e549a0ac17a393ae8617b",
          "time": 1638950949,
          "bits": 503385436,
          "nonce": 351770
        },
        "000000b7d508273169da2e0f1c167554d3d4643759361ab8ca338cabbbf5dee4": {
          "hash": "000000b7d508273169da2e0f1c167554d3d4643759361ab8ca338cabbbf5dee4",
          "version": 536870912,
          "prevHash": "0000009b0bb17530c41a8200868e61e8a2d927b3cce7cfac14c3f393ffca5b06",
          "merkleRoot": "664d055f0194e01ac64db1ace3a720f4580533d6d999b1cfab7740f8b9c4d642",
          "time": 1638884848,
          "bits": 503373836,
          "nonce": 13494
        },
      },
      "mappedBlockHeaderHeights": {
        "627561": "00000145d8a9ab1a71ffdcd01657d284c67593b27f9ca244fc202632bad7145d",
        "627574": "000000801bafea9630d9c6a341963912e7faff067e79b8b9b57910b62f736d3e",
        "627626": "00000053b6c3b0b56f64607ad77524922fbb4560306fa31f165ae36e11551d77"
      },
      "blockHeight": 627626,
      "fees": {
        "minRelay": 1000
      }
    }
  },
  "instantLocks": {
    "d37eb9f1b41c45134926d418486f2d8f57778cb949f4069a0b4c91955934913b": {
      "inputs": [
        {
          "outpointHash": "3188b2bfb238cb07c1822944088fff59d665af1f00a72d2beee8b88457201beb",
          "outpointIndex": 1
        }
      ],
      "txid": "d37eb9f1b41c45134926d418486f2d8f57778cb949f4069a0b4c91955934913b",
      "signature": "0628537fd9b44b5d2441724780183bea34f44e82fea7a7057337c562858bc9dabfe3541c8a7c9bd4677887dfaa2b02b90c483fe223590a05d6ab8670221aacf046731aa462a8e9f08271434ed6cbfaa1ec51c2be71f6e0a7d9612795b1a0f050"
    },
    "26faad59f6f3aef120323fb3aa9b0a3aaccf56ae770ff83099e3e86324d3a0da": {
      "inputs": [
        {
          "outpointHash": "b5cdbdae1f43ef324326b5519e2e670afa9203e2f37115c3c6ac51db968b3726",
          "outpointIndex": 1
        }
      ],
      "txid": "26faad59f6f3aef120323fb3aa9b0a3aaccf56ae770ff83099e3e86324d3a0da",
      "signature": "913ab2cf9bb6d918f0d2d12c5fef9ba9652ce93f625cebd83241746b36bdc6505e0385e1875576a92a5d49d74ec8098315e90c8688ccd41ab4fdc8df43bd83fa53081a65651bc30ca2957e4c2e98388465b19fac7b61f7b86393d777c2d26e66"
    },
  }
}


describe('Storage - rehydrateState', () => {
  const storage = {
    rehydrate: true,
    lastRehydrate: null,
    adapter: {
      getItem: async (key) => {
        return storeMock[key];
      }
    },
    store: {
      transactions: {},
      wallets: {},
      chains: {},
      instantLocks: {}
    },
    emit: () => {}
  }


  it('should rehydrate the state', async () => {
    await rehydrateState.call(storage);
    Object.values(storage.store.transactions).forEach(tx => {
      expect(tx instanceof Transaction).to.be.true;
    });

    Object.values(storage.store.instantLocks).forEach(isLock => {
      expect(isLock instanceof InstantLock).to.be.true;
    });

    Object.values(storage.store.wallets).forEach(wallet => {
      const { internal, external } = wallet.addresses;
      Object.values({ ...internal, ...external }).forEach(address => {
        Object.values(address.utxos).forEach(utxo => {
          expect(utxo instanceof Transaction.UnspentOutput).to.be.true;
        });
      });
    });

    Object.values(storage.store.chains).forEach(chain => {
      Object.values(chain.blockHeaders).forEach(blockHeader => {
        expect(blockHeader instanceof BlockHeader).to.be.true;
      })
    })
  });
});
