const _ = require('lodash');
const {expect} = require('chai');
const ChainStore = require('../../src/types/ChainStore/ChainStore');
const castItemTypes = require('./castStorageItemsTypes');
const {BlockHeader, Transaction} = require('@dashevo/dashcore-lib')
const WalletStore = require("../types/WalletStore/WalletStore");

const mockChainStorage = {
  "blockHeaders": {
    "fakeBlockHash": "000000206ff6709b4816a98a7601bc9626a597191fe4f788228a037de3bd839e811f4913c49de57ac9553f0cd2529c94eaa9781273f315a792c4148eb7e3756c9cd7e1ce40285562ffff7f2000000000"
  },
  "transactions": {
    "fakeTxHash": "0300000001e569f827418be2e49f5ae4a34d30ff3bc5abc723f72b0e10712598dff1e70689010000006b483045022100f9aebe9bcfaa8208f1486ad4d15f65730b5adc0cb02c1d2bd8836a4582e2ea7f02200250bbfe524f70324417237549f421c2c3584d6b532194dbac8043e46900753701210387f3d1ff9e6a06db60bd61d0757002836407e8a3b1094b446690d13392c3fb9affffffff02e8030000000000001976a9148d0ba6247ad4988ba70fdcb56bb5f45b6e49423d88aca351c253040000001976a91471a97f71915e7c1d4460ad520511761b06534d8088ac00000000"
  },
  "instantLocks": {},
  "txMetadata": {
    "fakeTxHash": {
      "blockHash": "0eca27f921836079a85f41b134679cd557fab1f66b2e60013b873eb56e7b3f2d",
      "height": 5409,
      "isInstantLocked": true,
      "isChainLocked": true
    }
  },
  "fees": {
    "minRelay": -1
  }
}

const mockWalletStorage = {
  "lastKnownBlock": {
    "height": 11703
  }
}

describe('Utils - castItemTypes', function suite() {
    it('should proceed with valid schema', function () {
      const chainSchema = castItemTypes(mockChainStorage, ChainStore.prototype.SCHEMA)

      expect(chainSchema.blockHeaders.fakeBlockHash instanceof BlockHeader).to.be.true
      expect(chainSchema.transactions.fakeTxHash instanceof Transaction).to.be.true
      expect(chainSchema.txMetadata.fakeTxHash.isInstantLocked).to.be.true
      expect(chainSchema.txMetadata.fakeTxHash.isChainLocked).to.be.true
      expect(typeof chainSchema.txMetadata.fakeTxHash.height).to.be.equal('number')

      const walletSchema = castItemTypes(mockWalletStorage, WalletStore.prototype.SCHEMA)

      expect(walletSchema.lastKnownBlock.height).to.be.equal(11703)
    });

    it('should omit unknown keys in storage data', function () {
      const mockChainStorageWithUnknownKeys = _.cloneDeep(mockChainStorage)
      mockChainStorageWithUnknownKeys.txMetadata.unknownKey = true

      const schema = castItemTypes(mockChainStorage, ChainStore.prototype.SCHEMA);

      expect(schema.txMetadata.unknownKey).to.be.equal(undefined)
    });

    it('should throw if no schema passed', function () {
      expect(() => castItemTypes(mockChainStorage, null))
        .to.throw(Error, 'Schema is undefined')
    });

    it('should throw if some of the keys contains wrong type', function () {
      const mockWalletStorageWithWrongType = _.cloneDeep(mockWalletStorage)
      mockWalletStorageWithWrongType.lastKnownBlock.height = '11703'
      expect(() => castItemTypes(mockWalletStorageWithWrongType, WalletStore.prototype.SCHEMA))
        .to.throw(Error, 'Value "11703" is not of type "number"');
    });

    it('should throw if some of the keys are missing from the storage', function () {
      const mockWalletChainStorageWithMissingKeys = _.cloneDeep(mockChainStorage)
      mockWalletChainStorageWithMissingKeys.txMetadata = undefined

      expect(() => castItemTypes(mockWalletChainStorageWithMissingKeys, ChainStore.prototype.SCHEMA))
        .to.throw(Error, 'No item found for schema key "txMetadata" in item');
    });
  }
);
