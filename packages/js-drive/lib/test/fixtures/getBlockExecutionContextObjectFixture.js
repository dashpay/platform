const {
  tendermint: {
    abci: {
      CommitInfo,
      ValidatorSetUpdate,
    },
    types: {
      ConsensusParams,
    },
  },
} = require('@dashevo/abci/types');

const pino = require('pino');

const getDataContractFixture = require('@dashevo/dpp/lib/test/fixtures/getDataContractFixture');
const { hash } = require('@dashevo/dpp/lib/util/hash');

/**
 * @param {DataContract} [dataContract]
 * @return {{
 *   dataContracts: Object[],
 *   lastCommitInfo,
 *   coreChainLockedHeight: number,
 *   height: number,
 *   version: number,
 *   timeMs: number,
 *   validTxs: number,
 *   consensusLogger: Logger,
 *   withdrawalTransactionsMap: Object,
 *   round: number,
 * }}
 */
function getBlockExecutionContextObjectFixture(dataContract = getDataContractFixture()) {
  const lastCommitInfo = new CommitInfo({
    quorumHash: Buffer.from('000003c60ecd9576a05a7e15d93baae18729cb4477d44246093bd2cf8d4f53d8', 'hex'),
    blockSignature: Buffer.from('003657bb44d74c371d14485117de43313ca5c2848f3622d691c2b1bf3576a64bdc2538efab24854eb82ae7db38482dbd15a1cb3bc98e55173817c9d05c86e47a5d67614a501414aae6dd1565e59422d1d77c41ae9b38de34ecf1e9f778b2a97b', 'hex'),
  });

  const version = {
    app: '1',
    block: '2',
  };

  const [txOneBytes, txTwoBytes] = [
    Buffer.alloc(32, 0),
    Buffer.alloc(32, 1),
  ];

  return {
    dataContracts: [dataContract.toObject()],
    lastCommitInfo: CommitInfo.toObject(lastCommitInfo),
    height: 10,
    coreChainLockedHeight: 10,
    version,
    consensusLogger: pino(),
    epochInfo: {
      height: 1,
      timeMs: 100,
      epoch: 0,
    },
    timeMs: Date.now(),
    withdrawalTransactionsMap: {
      [hash(txOneBytes).toString('hex')]: txOneBytes,
      [hash(txTwoBytes).toString('hex')]: txTwoBytes,
    },
    round: 42,
    prepareProposalResult: {
      appHash: Buffer.alloc(32, 3),
      txResults: new Array(3).fill({ code: 0 }),
      consensusParamUpdates: new ConsensusParams({
        block: {
          maxBytes: 1,
          maxGas: 2,
        },
        evidence: {
          maxAgeDuration: null,
          maxAgeNumBlocks: 1,
          maxBytes: 2,
        },
        version: {
          appVersion: 1,
        },
      }),
      validatorSetUpdate: new ValidatorSetUpdate(),
      coreChainLockUpdate: {
        coreBlockHeight: 42,
        coreBlockHash: '1528e523f4c20fa84ba70dd96372d34e00ce260f357d53ad1a8bc892ebf20e2d',
        signature: '1897ce8f54d2070f44ca5c29983b68b391e8137c25e44f67416e579f3e3bdfef7b4fd22db7818399147e52907998857b0fbc8edfdc40a64f2c7df0e88544d31d12ca8c15e73d50dda25ca23f754ed3f789ed4bcb392161995f464017c10df404',
      },
      txRecords: [{
        tx: Buffer.alloc(5, 0),
        action: 1,
      }],
    },
  };
}

module.exports = getBlockExecutionContextObjectFixture;
