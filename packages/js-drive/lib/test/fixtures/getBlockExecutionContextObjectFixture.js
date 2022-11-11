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
const { hash } = require('@dashevo/dpp/lib/util/hash');

/**
 * @param {DataContract} [dataContract]
 * @return {{
 *   dataContracts: Object[],
 *   lastCommitInfo,
 *   coreChainLockedHeight: number,
 *   height: number,
 *   version: number,
 *   time: Timestamp,
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
    stateSignature: Buffer.from('09c3e46f5bc1abcb7c130b8c36a168e1fbc471fa86445dfce49e151086a277216e7a5618a7554b823d995c5606d0642f18f9c4caa249605d2ab156e14728c82f58f9008d4bcc6e21e0a561e3185e2ae654605613e86af507ca49079595872532', 'hex'),
  });

  const time = new Timestamp({
    seconds: Math.ceil(new Date().getTime() / 1000),
    nanos: 0,
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
    time,
    height: 10,
    coreChainLockedHeight: 10,
    version,
    consensusLogger: pino(),
    withdrawalTransactionsMap: {
      [hash(txOneBytes).toString('hex')]: txOneBytes,
      [hash(txTwoBytes).toString('hex')]: txTwoBytes,
    },
    round: 42,
  };
}

module.exports = getBlockExecutionContextObjectFixture;
