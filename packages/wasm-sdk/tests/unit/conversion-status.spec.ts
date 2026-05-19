import { expect } from './helpers/chai.ts';
import init, * as sdk from '../../dist/sdk.compressed.js';

describe('StatusResponse Conversions', () => {
  before(async () => {
    await init();
  });

  const statusFixture = {
    version: {
      software: { dapi: '1.5.0', drive: '1.5.0', tenderdash: '0.14.0' },
      protocol: {
        tenderdash: { p2p: 8, block: 13 },
        drive: { latest: 7, current: 7 },
      },
    },
    node: { id: 'abcdef1234567890', proTxHash: '1234abcd' },
    chain: {
      isCatchingUp: false,
      latestBlockHash: 'aabb',
      latestAppHash: 'ccdd',
      latestBlockHeight: '12345',
      earliestBlockHash: '0000',
      earliestAppHash: '0000',
      earliestBlockHeight: '1',
      maxPeerBlockHeight: '12345',
      coreChainLockedHeight: 1000,
    },
    network: { chainId: 'dash-testnet-51', peersCount: 10, isListening: true },
    stateSync: {
      totalSyncedTime: '0',
      remainingTime: '0',
      totalSnapshots: 0,
      chunkProcessAvgTime: '0',
      snapshotHeight: '0',
      snapshotChunksCount: '0',
      backfilledBlocks: '0',
      backfillBlocksTotal: '0',
    },
    time: {
      local: '2024-01-01T00:00:00Z',
      block: '2024-01-01T00:00:00Z',
      genesis: '2023-01-01T00:00:00Z',
      epoch: 10,
    },
  };

  describe('fromJSON()', () => {
    it('should deserialize from JSON fixture', () => {
      const status = sdk.StatusResponse.fromJSON(statusFixture);

      expect(status.version).to.exist();
      expect(status.version.software.dapi).to.equal('1.5.0');
      expect(status.version.software.drive).to.equal('1.5.0');
      expect(status.version.software.tenderdash).to.equal('0.14.0');
      expect(status.version.protocol.tenderdash.p2p).to.equal(8);
      expect(status.version.protocol.tenderdash.block).to.equal(13);
      expect(status.version.protocol.drive.latest).to.equal(7);
      expect(status.version.protocol.drive.current).to.equal(7);

      expect(status.node.id).to.equal('abcdef1234567890');

      expect(status.chain.isCatchingUp).to.equal(false);
      expect(status.chain.latest_block_height).to.equal('12345');
      expect(status.chain.core_chain_locked_height).to.equal(1000);

      expect(status.network.chain_id).to.equal('dash-testnet-51');
      expect(status.network.peers_count).to.equal(10);
      expect(status.network.isListening).to.equal(true);

      expect(status.time.local).to.equal('2024-01-01T00:00:00Z');
      expect(status.time.block).to.equal('2024-01-01T00:00:00Z');
      expect(status.time.genesis).to.equal('2023-01-01T00:00:00Z');
      expect(status.time.epoch).to.equal(10);

      expect(status.state_sync.total_snapshots).to.equal(0);

      status.free();
    });
  });

  describe('toJSON()', () => {
    it('should round-trip through JSON', () => {
      const status = sdk.StatusResponse.fromJSON(statusFixture);
      const json = status.toJSON();
      const restored = sdk.StatusResponse.fromJSON(json);
      const json2 = restored.toJSON();

      expect(json2).to.deep.equal(json);

      status.free();
      restored.free();
    });

    it('should serialize version.software fields', () => {
      const status = sdk.StatusResponse.fromJSON(statusFixture);
      const json = status.toJSON();

      expect(json.version.software.dapi).to.equal('1.5.0');
      expect(json.version.software.drive).to.equal('1.5.0');
      expect(json.version.software.tenderdash).to.equal('0.14.0');

      status.free();
    });

    it('should serialize version.protocol fields', () => {
      const status = sdk.StatusResponse.fromJSON(statusFixture);
      const json = status.toJSON();

      expect(json.version.protocol.tenderdash.p2p).to.equal(8);
      expect(json.version.protocol.tenderdash.block).to.equal(13);
      expect(json.version.protocol.drive.latest).to.equal(7);
      expect(json.version.protocol.drive.current).to.equal(7);

      status.free();
    });

    it('should serialize chain fields with camelCase keys', () => {
      const status = sdk.StatusResponse.fromJSON(statusFixture);
      const json = status.toJSON();

      expect(json.chain.isCatchingUp).to.equal(false);
      expect(json.chain.latestBlockHash).to.equal('aabb');
      expect(json.chain.latestAppHash).to.equal('ccdd');
      expect(json.chain.latestBlockHeight).to.equal('12345');
      expect(json.chain.earliestBlockHash).to.equal('0000');
      expect(json.chain.earliestAppHash).to.equal('0000');
      expect(json.chain.earliestBlockHeight).to.equal('1');
      expect(json.chain.maxPeerBlockHeight).to.equal('12345');
      expect(json.chain.coreChainLockedHeight).to.equal(1000);

      status.free();
    });

    it('should serialize network fields', () => {
      const status = sdk.StatusResponse.fromJSON(statusFixture);
      const json = status.toJSON();

      expect(json.network.chainId).to.equal('dash-testnet-51');
      expect(json.network.peersCount).to.equal(10);
      expect(json.network.isListening).to.equal(true);

      status.free();
    });

    it('should serialize stateSync fields', () => {
      const status = sdk.StatusResponse.fromJSON(statusFixture);
      const json = status.toJSON();

      expect(json.stateSync.totalSyncedTime).to.equal('0');
      expect(json.stateSync.remainingTime).to.equal('0');
      expect(json.stateSync.totalSnapshots).to.equal(0);

      status.free();
    });

    it('should serialize time fields', () => {
      const status = sdk.StatusResponse.fromJSON(statusFixture);
      const json = status.toJSON();

      expect(json.time.local).to.equal('2024-01-01T00:00:00Z');
      expect(json.time.block).to.equal('2024-01-01T00:00:00Z');
      expect(json.time.genesis).to.equal('2023-01-01T00:00:00Z');
      expect(json.time.epoch).to.equal(10);

      status.free();
    });
  });

  describe('toObject()', () => {
    it('should round-trip through Object', () => {
      const status = sdk.StatusResponse.fromJSON(statusFixture);
      const obj = status.toObject();
      const restored = sdk.StatusResponse.fromObject(obj);
      const obj2 = restored.toObject();

      expect(obj2).to.deep.equal(obj);

      status.free();
      restored.free();
    });
  });

  describe('fromObject()', () => {
    it('should round-trip through toObject/fromObject preserving getters', () => {
      const status = sdk.StatusResponse.fromJSON(statusFixture);
      const obj = status.toObject();
      const restored = sdk.StatusResponse.fromObject(obj);

      expect(restored.version.software.dapi).to.equal('1.5.0');
      expect(restored.chain.latest_block_height).to.equal('12345');
      expect(restored.network.chain_id).to.equal('dash-testnet-51');
      expect(restored.time.local).to.equal('2024-01-01T00:00:00Z');

      status.free();
      restored.free();
    });
  });

  describe('optional fields', () => {
    it('should handle missing optional fields in chain', () => {
      const fixtureWithoutOptional = {
        ...statusFixture,
        chain: {
          isCatchingUp: false,
          latestBlockHash: 'aabb',
          latestAppHash: 'ccdd',
          latestBlockHeight: '100',
          earliestBlockHash: '0000',
          earliestAppHash: '0000',
          earliestBlockHeight: '1',
          maxPeerBlockHeight: '100',
        },
      };

      const status = sdk.StatusResponse.fromJSON(fixtureWithoutOptional);
      expect(status.chain.core_chain_locked_height).to.be.undefined();

      status.free();
    });

    it('should handle missing optional fields in time', () => {
      const fixtureWithMinimalTime = {
        ...statusFixture,
        time: {
          local: '2024-01-01T00:00:00Z',
        },
      };

      const status = sdk.StatusResponse.fromJSON(fixtureWithMinimalTime);
      expect(status.time.local).to.equal('2024-01-01T00:00:00Z');
      expect(status.time.block).to.be.undefined();
      expect(status.time.genesis).to.be.undefined();
      expect(status.time.epoch).to.be.undefined();

      status.free();
    });

    it('should handle missing optional fields in software', () => {
      const fixtureWithMinimalSoftware = {
        ...statusFixture,
        version: {
          ...statusFixture.version,
          software: { dapi: '1.5.0' },
        },
      };

      const status = sdk.StatusResponse.fromJSON(fixtureWithMinimalSoftware);
      expect(status.version.software.dapi).to.equal('1.5.0');
      expect(status.version.software.drive).to.be.undefined();
      expect(status.version.software.tenderdash).to.be.undefined();

      status.free();
    });
  });

  describe('sub-type conversions', () => {
    describe('StatusVersion', () => {
      it('should round-trip through JSON', () => {
        const version = sdk.StatusVersion.fromJSON(statusFixture.version);
        const json = version.toJSON();
        const restored = sdk.StatusVersion.fromJSON(json);
        const json2 = restored.toJSON();

        expect(json2).to.deep.equal(json);

        version.free();
        restored.free();
      });
    });

    describe('StatusChain', () => {
      it('should round-trip through JSON', () => {
        const chain = sdk.StatusChain.fromJSON(statusFixture.chain);
        const json = chain.toJSON();
        const restored = sdk.StatusChain.fromJSON(json);
        const json2 = restored.toJSON();

        expect(json2).to.deep.equal(json);

        chain.free();
        restored.free();
      });
    });

    describe('StatusNetwork', () => {
      it('should round-trip through JSON', () => {
        const network = sdk.StatusNetwork.fromJSON(statusFixture.network);
        const json = network.toJSON();
        const restored = sdk.StatusNetwork.fromJSON(json);
        const json2 = restored.toJSON();

        expect(json2).to.deep.equal(json);

        network.free();
        restored.free();
      });
    });

    describe('StatusTime', () => {
      it('should round-trip through JSON', () => {
        const time = sdk.StatusTime.fromJSON(statusFixture.time);
        const json = time.toJSON();
        const restored = sdk.StatusTime.fromJSON(json);
        const json2 = restored.toJSON();

        expect(json2).to.deep.equal(json);

        time.free();
        restored.free();
      });
    });
  });
});
