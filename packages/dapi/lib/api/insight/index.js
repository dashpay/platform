const requestPromiseNative = require('request-promise-native');
const querystring = require('querystring');
const MockListGenerator = require('../../mocks/dynamicMnList');
const config = require('../../config/index');

const URI = config.insightUri;
const mnListGenerator = new MockListGenerator();

const request = async (uri, method, data = {}) => {
  const fullURI = `${URI}${uri}`;
  let response;
  if (method === 'GET') {
    const query = querystring.stringify(data);
    response = await requestPromiseNative.get(fullURI, { json: true, qs: query });
  } else if (method === 'POST') {
    response = await requestPromiseNative.post(fullURI, { json: true, body: data });
  } else {
    throw new Error(`Wrong method: ${method}`);
  }
  if (typeof response === 'string') {
    throw new Error(response);
  }
  if (response.error) {
    throw new Error(response.error);
  }
  if (!response.result) {
    // Some insight methods returns data that way
    return response;
  }
  return response.result;
};

const get = async (uri, data) => request(uri, 'GET', data);
const post = async (uri, data) => request(uri, 'POST', data);

const getTransactionFirstInputAddress = async (txHash) => {
  const res = await get(`/tx/${txHash}`);
  return res.vin[0].addr;
};

const getHashFromHeight = async (height) => {
  const res = await get(`/block-index/${height}`);
  return res.blockHash;
};

const getMnList = () => mnListGenerator.getMockMnList();
const getMnUpdateList = () => mnListGenerator.getMockMnUpdateList();
const getUTXO = async address => get(`/addr/${address}/utxo`);
const getBalance = async address => get(`/addr/${address}/balance`);
const getUser = async usernameOrRegTx => get(`/getuser/${usernameOrRegTx}`);
const getMasternodesList = async () => get('/masternodes/list');

const getBestBlockHeight = async () => {
  const res = await get('/status');
  return res.info.blocks;
};

const getBlockHash = async (blockHeight) => {
  const res = await get(`/block-index/${blockHeight}`);
  return res.blockHash;
};

const getAddressTotalReceived = async (address) => {
  const res = await get(`/addr/${address}/totalReceived`);
  return res;
};

const getBlocks = async (blockDate, limit) => {
  let queryParams = '?';
  if (blockDate) {
    queryParams += `blockDate=${blockDate}&`;
  }
  if (limit) {
    queryParams += `limit=${limit}`;
  } else {
    queryParams = queryParams.slice(0, -1);
  }
  const res = await get(`/blocks${queryParams}`);
  return res.blocks;
};

const getAddressTotalSent = async (address) => {
  const res = await get(`/addr/${address}/totalSent`);
  return res;
};

const getAddressUnconfirmedBalance = async (address) => {
  const res = await get(`/addr/${address}/unconfirmedBalance`);
  return res;
};

const getAddressSummary = async (address) => {
  const res = await get(`/addr/${address}`);
  return res;
};

const getRawBlock = async (blockHash) => {
  const res = await get(`/rawblock/${blockHash}`);
  return res;
};

const getTransactionsByAddress = async (address) => {
  const res = await get(`/addrs/${address}/txs`); // TODO addrs instead fo addr?
  return res;
};

const getHistoricBlockchainDataSyncStatus = async () => {
  const res = await get('/sync');
  return res;
};

const getStatus = async (queryString) => {
  switch (queryString) {
    case 'getInfo':
      break;
    case 'getDifficulty':
      break;
    case 'getBestBlockHash':
      break;
    case 'getLastBlockHash':
      break;
    default:
      throw new Error('Invalid query string.');
  }
  const res = await get(`/status?q=${queryString}`);
  return res;
};

const getPeerDataSyncStatus = async () => {
  const res = await get('/peer');
  return res;
};

// eslint-disable-next-line no-unused-vars
const estimateFee = async blocks => 1; // 1 duff per byte

const getTransactionById = async (txid) => {
  const res = await get(`/tx/${txid}`);
  return res;
};

const getBlockHeaders = async (offset, limit) => get(`/block-headers/${offset}/${limit}`);

module.exports = {
  getTransactionFirstInputAddress,
  getHashFromHeight,
  getMnList,
  getMnUpdateList,
  request,
  get,
  post,
  getUTXO,
  getBalance,
  getUser,
  getBestBlockHeight,
  getBlockHash,
  getMasternodesList,
  getAddressTotalReceived,
  getBlocks,
  getAddressTotalSent,
  getAddressUnconfirmedBalance,
  getAddressSummary,
  getRawBlock,
  getTransactionsByAddress,
  getHistoricBlockchainDataSyncStatus,
  getStatus,
  getPeerDataSyncStatus,
  estimateFee,
  getTransactionById,
  getBlockHeaders,
};
