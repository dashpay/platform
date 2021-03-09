const RpcClient = require('@dashevo/dashd-rpc');
const DashCoreRpcError = require('../../errors/DashCoreRpcError');
const constants = require('./constants');
const config = require('../../config');

const client = new RpcClient(config.dashcore.rpc);

/**
 *  Layer 1 endpoints
 *  These functions represent endpoints on the transactional layer
 *  and can be requested from any random DAPI node.
 *  Once a DAPI-client is assigned to a quorum it should exclude its quorum nodes
 *  from the set of nodes serving L1 endpoints for privacy reasons
 */

function generateToAddress(blocksNumber, address) {
  return new Promise((resolve, reject) => { // not exist?
    client.generateToAddress(blocksNumber, address, (err, r) => {
      if (err) {
        reject(new DashCoreRpcError(err.message, null, err.code));
      } else {
        resolve(r.result);
      }
    });
  });
}

const getBestBlockHash = () => new Promise((resolve, reject) => {
  client.getbestblockhash((err, r) => {
    if (err) {
      reject(new DashCoreRpcError(err.message, null, err.code));
    } else {
      resolve(r.result);
    }
  });
});

const getBestBlockHeight = () => new Promise((resolve, reject) => {
  client.getblockcount((err, r) => {
    if (err) {
      reject(new DashCoreRpcError(err.message, null, err.code));
    } else {
      resolve(r.result);
    }
  });
});

const getBlock = (hash, isParsed = 1) => new Promise((resolve, reject) => {
  client.getblock(hash, isParsed, (err, r) => {
    if (err) {
      reject(new DashCoreRpcError(err.message, null, err.code));
    } else {
      resolve(r.result);
    }
  });
});

const getBlockHash = index => new Promise((resolve, reject) => {
  client.getblockhash(index, (err, r) => {
    if (err) {
      reject(new DashCoreRpcError(err.message, null, err.code));
    } else {
      resolve(r.result);
    }
  });
});

const getBlockHeader = blockHash => new Promise((resolve, reject) => {
  client.getblockheader(blockHash, (err, r) => {
    if (err) {
      reject(new DashCoreRpcError(err.message, null, err.code));
    } else {
      resolve(r.result);
    }
  });
});

const getBlockHeaders = (offset, limit = 1, verbose = false) => new Promise((resolve, reject) => {
  client.getblockheaders(offset, limit, verbose, (err, r) => {
    if (err) {
      reject(new DashCoreRpcError(err.message, null, err.code));
    } else {
      resolve(r.result);
    }
  });
});

const getMasternodesList = () => new Promise((resolve, reject) => {
  client.masternodelist((err, r) => {
    if (err) {
      reject(new DashCoreRpcError(err.message, null, err.code));
    } else {
      resolve(r.result);
    }
  });
});

const getMempoolInfo = () => new Promise((resolve, reject) => {
  client.getmempoolinfo((err, r) => {
    if (err) {
      reject(new DashCoreRpcError(err.message, null, err.code));
    } else {
      resolve(r.result);
    }
  });
});

const getMnListDiff = (baseBlockHash, blockHash) => new Promise((resolve, reject) => {
  client.protx(constants.DASHCORE_RPC_COMMANDS.protx.diff, baseBlockHash, blockHash, (err, r) => {
    if (err) {
      reject(new DashCoreRpcError(err.message, null, err.code));
    } else {
      resolve(r.result);
    }
  });
});

/*eslint-disable */
// Temporary mock result
const getQuorum = regtxid => new Promise((resolve, reject) => {

  //remove when rpc getQuorum available
  const coreFixtures = require('../../../test/mocks/coreAPIFixture');
  coreFixtures.getQuorum(regtxid)
    .then(mockData => resolve(mockData))

  // re-add when rpc getQuorum available
  // client.getquorum(regtxid, (err, r) => {
  //   if (err) {
  //     reject(new DashCoreRpcError(err.message, null, err.code));
  //   } else {
  //     resolve(r.result);
  //   }
  // });
});
/* eslint-enable */

const getRawTransaction = txid => new Promise((resolve, reject) => {
  client.getrawtransaction(txid, (err, r) => {
    if (err) {
      reject(new DashCoreRpcError(err.message, null, err.code));
    } else {
      resolve(r.result);
    }
  });
});

const getRawBlock = txid => getBlock(txid, false);

// This is only for in-wallet transaction
const getTransaction = txid => new Promise((resolve, reject) => {
  client.gettransaction(txid, (err, r) => {
    if (err) {
      reject(new DashCoreRpcError(err.message, null, err.code));
    } else {
      resolve(r.result);
    }
  });
});

const getTransactionFirstInputAddress = txId => new Promise((resolve, reject) => {
  client.gettransaction(txId, (err, r) => {
    if (err) {
      reject(new DashCoreRpcError(err.message, null, err.code));
    } else {
      resolve(r.details.address);
    }
  });
});

const getUser = txId => new Promise((resolve, reject) => { // not exist?
  client.getuser(txId, (err, r) => {
    if (err) {
      reject(new DashCoreRpcError(err.message, null, err.code));
    } else {
      resolve(r.result);
    }
  });
});

// Address indexing needs to be enabled
const getUTXO = addr => new Promise((resolve, reject) => {
  client.getaddressutxos(addr, (err, r) => {
    if (err) {
      reject(new DashCoreRpcError(err.message, null, err.code));
    } else {
      resolve(r.result);
    }
  });
});

/**
 *
 * @param {string} bloomFilter - hex string representing serialized bloom filter
 * @param {string} fromBlockHash - block hash as a hex string
 * @param {number} [count] - how many blocks to scan. Max 2000
 * @return {Promise<string[]>} - serialized merkle blocks
 */
const getMerkleBlocks = (bloomFilter, fromBlockHash, count) => new Promise((resolve, reject) => {
  client.getMerkleBlocks(bloomFilter, fromBlockHash, count, (error, response) => {
    if (error) {
      reject(new DashCoreRpcError(error.message));
    } else {
      resolve(response.result);
    }
  });
});

/**
 *  Layer 2 endpoints
 *  These functions represent endpoints on the data layer
 *  and can be requested only from members of the quorum assigned to a specific DAPI-client
 */

const sendRawTransition = ts => new Promise((resolve, reject) => { // not exist?
  client.sendrawtransition(ts, (err, r) => {
    if (err) {
      reject(new DashCoreRpcError(err.message, null, err.code));
    } else {
      resolve(r.result);
    }
  });
});

/**
 *  Layer 1 or Layer 2 endpoints
 *  depending on context these functions are either Layer 1 or Layer 2
 *  e.g. sendRawTransaction can be used to send a normal tx => Layer 1,
 *  but can also be used like its alias sendRawTransition to send
 *  a state transition updating a BU account => Layer 2.
 *  A DAPI-client will need to know if it has already been assigned
 *  a quorum in order to choose which set of DAPI nodes to use
 *  for posting a tx to this endpoint -
 *  all DAPI nodes or just it's quorum member nodes
 */

const sendRawTransaction = tx => new Promise((resolve, reject) => {
  client.sendrawtransaction(tx, (err, r) => {
    if (err) {
      reject(new DashCoreRpcError(err.message, null, err.code));
    } else {
      resolve(r.result);
    }
  });
});

const sendRawIxTransaction = tx => new Promise((resolve, reject) => {
  client.sendrawtransaction(tx, false, true, (err, r) => {
    if (err) {
      reject(new DashCoreRpcError(err.message, null, err.code));
    } else {
      resolve(r.result);
    }
  });
});

/**
 * @typedef CoreRpcClient
 * @type {{
 * getMempoolInfo: (function(): Promise<any>),
 * sendRawTransaction: (function(*=): Promise<any>),
 * getBlock: (function(*=, *=): Promise<any>),
 * getUser: (function(*=): Promise<any>),
 * getUTXO: (function(*=): Promise<any>),
 * getBlockHash: (function(*=): Promise<any>),
 * getBestBlockHash: (function(): Promise<any>),
 * getMnListDiff: (function(*=, *=): Promise<any>),
 * getBlockHeaders: (function(*=, *=, *=): Promise<any>),
 * getRawTransaction: (function(*=): Promise<any>),
 * getTransactionFirstInputAddress: (function(*=): Promise<any>),
 * getBlockHeader: (function(*=): Promise<any>),
 * sendRawIxTransaction: (function(*=): Promise<any>),
 * getRawBlock: (function(*=): (*|Promise<any>)),
 * getQuorum: (function(*=): Promise<any>),
 * getMasternodesList: (function(): Promise<any>),
 * getBestBlockHeight: (function(): Promise<any>),
 * sendRawTransition: (function(*=): Promise<any>),
 * generateToAddress: (function(*=): Promise<any>),
 * getTransaction: (function(*=): Promise<any>),
 * getMerkleBlocks: (function(string, string, number): Promise<string[]>)}}
 */
module.exports = {
  generateToAddress,
  getBestBlockHash,
  getBestBlockHeight,
  getBlockHash,
  getBlock,
  getBlockHeader,
  getBlockHeaders,
  getMasternodesList,
  getMempoolInfo,
  getMnListDiff,
  getQuorum,
  sendRawTransition,
  sendRawTransaction,
  sendRawIxTransaction,
  getRawTransaction,
  getRawBlock,
  getTransaction,
  getTransactionFirstInputAddress,
  getUser,
  getUTXO,
  getMerkleBlocks,
};
