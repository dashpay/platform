const {
  tendermint: {
    abci: {
      CommitInfo,
    },
  },
  google: {
    protobuf: {
      Timestamp,
    },
  },
} = require('@dashevo/abci/types');

const pino = require('pino');

const getDataContractFixture = require('@dashevo/dpp/lib/test/fixtures/getDataContractFixture');

/**
 * @param {DataContract} [dataContract]
 * @return {{
 *   dataContracts: Object[],
 *   lastCommitInfo,
 *   invalidTxs: number,
 *   coreChainLockedHeight: number,
 *   height: number,
 *   version: number,
 *   time: number,
 *   validTxs: number,
 *   cumulativeFees: number,
 *   consensusLogger: Logger,
 * }}
 */
function getBlockExecutionContextObjectFixture(dataContract = getDataContractFixture()) {
  const lastCommitInfo = new CommitInfo({
    quorumHash: Buffer.from('000003c60ecd9576a05a7e15d93baae18729cb4477d44246093bd2cf8d4f53d8', 'hex'),
    blockSignature: Buffer.from('003657bb44d74c371d14485117de43313ca5c2848f3622d691c2b1bf3576a64bdc2538efab24854eb82ae7db38482dbd15a1cb3bc98e55173817c9d05c86e47a5d67614a501414aae6dd1565e59422d1d77c41ae9b38de34ecf1e9f778b2a97b', 'hex'),
    stateSignature: Buffer.from('09c3e46f5bc1abcb7c130b8c36a168e1fbc471fa86445dfce49e151086a277216e7a5618a7554b823d995c5606d0642f18f9c4caa249605d2ab156e14728c82f58f9008d4bcc6e21e0a561e3185e2ae654605613e86af507ca49079595872532', 'hex'),
  });

  const time = new Timestamp({
    seconds: Math.ceil(new Date().getTime() / 1000),
    nanos: 0,
  });

  const height = 10;

  const coreChainLockedHeight = 10;

  const version = {
    app: '1',
    block: '2',
  };

  const cumulativeFees = 10;

  const logger = pino();

  const validTxs = 2;
  const invalidTxs = 1;

  return {
    dataContracts: [dataContract.toObject()],
    lastCommitInfo: CommitInfo.toObject(lastCommitInfo),
    cumulativeFees,
    time,
    height,
    coreChainLockedHeight,
    version,
    validTxs,
    invalidTxs,
    consensusLogger: logger,
  };
}

module.exports = getBlockExecutionContextObjectFixture;
