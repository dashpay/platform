const requestPromiseNative = require('request-promise-native');
const querystring = require('querystring');
const config = require('../../config/index');

const URI = config.insightUri;

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

const getUTXO = async (address, from, to, fromHeight, toHeight) => {
  const addresses = Array.isArray(address) ? address.join() : address;
  return get(`/addrs/${addresses}/utxopage?from=${from}&to=${to}&fromHeight=${fromHeight}&to=${toHeight}`);
};

const getBalance = async address => (
  Array.isArray(address)
    ? get(`/addrs/${address.join()}/balance`)
    : get(`/addr/${address}/balance`)
);

const getUser = async usernameOrRegTx => get(`/getuser/${usernameOrRegTx}`);
const getMasternodesList = async () => get('/masternodes/list');

const getBestBlockHeight = async () => {
  const { info: { blocks } } = await get('/status');

  return blocks;
};

const getAddressTotalReceived = async address => (
  Array.isArray(address)
    ? get(`/addrs/${address.join()}/totalReceived`)
    : get(`/addr/${address}/totalReceived`)
);

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

  const { blocks } = await get(`/blocks${queryParams}`);

  return blocks;
};

const getAddressTotalSent = async address => (
  Array.isArray(address)
    ? get(`/addrs/${address.join()}/totalSent`)
    : get(`/addr/${address}/totalSent`)
);

const getAddressUnconfirmedBalance = async address => (
  Array.isArray(address)
    ? get(`/addrs/${address.join()}/unconfirmedBalance`)
    : get(`/addr/${address}/unconfirmedBalance`)
);

const getAddressSummary = async (address, noTxList = false, from = '', to = '', fromHeight = '', toHeight = '') => (
  Array.isArray(address)
    ? get(`/addrs/${address.join()}?noTxList=${+noTxList}&from=${from}&to=${to}&fromHeight=${fromHeight}&to=${toHeight}`)
    : get(`/addr/${address}?noTxList=${+noTxList}&from=${from}&to=${to}&fromHeight=${fromHeight}&to=${toHeight}`)
);

const getTransactionsByAddress = async (address, from, to, fromHeight, toHeight) => {
  const addresses = Array.isArray(address) ? address.join() : address;
  return get(`/addrs/${addresses}/txs?from=${from}&to=${to}&fromHeight=${fromHeight}&to=${toHeight}`);
};

const getHistoricBlockchainDataSyncStatus = async () => get('/sync');

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

  return get(`/status?q=${queryString}`);
};

const getPeerDataSyncStatus = async () => get('/peer');

const estimateFee = async (blocks) => {
  let url = '/utils/estimatefee';

  if (blocks) {
    url = `${url}?nbBlocks=${blocks}`;
  }

  return get(url);
};

const getTransactionById = async txId => get(`/tx/${txId}`);

const getRawTransactionById = async (txId) => {
  const { rawtx } = await get(`/rawtx/${txId}`);

  return rawtx;
};

const getBlockHeaders = async (offset, limit) => get(`/block-headers/${offset}/${limit}`);

const getBlockByHash = async hash => get(`/block/${hash}`);

const getBlockByHeight = async (height) => {
  const res = await get(`/block-index/${height}`);
  const { blockHash: hash } = res;

  return getBlockByHash(hash);
};

const sendTransaction = async (rawtx) => {
  const { txid } = await post('/tx/send', {
    rawtx,
  });

  return txid;
};

const getRawBlockByHash = async (hash) => {
  const { rawblock } = await get(`/rawblock/${hash}`);

  return rawblock;
};

const getRawBlockByHeight = async (height) => {
  const res = await get(`/block-index/${height}`);
  const { blockHash: hash } = res;

  return getRawBlockByHash(hash);
};

/**
 * @typedef InsightAPI
 */
module.exports = {
  getTransactionFirstInputAddress,
  request,
  get,
  post,
  getUTXO,
  getBalance,
  getUser,
  getBestBlockHeight,
  getMasternodesList,
  getAddressTotalReceived,
  getBlocks,
  getAddressTotalSent,
  getAddressUnconfirmedBalance,
  getAddressSummary,
  getTransactionsByAddress,
  getHistoricBlockchainDataSyncStatus,
  getStatus,
  getPeerDataSyncStatus,
  estimateFee,
  getTransactionById,
  getBlockHeaders,
  getBlockByHeight,
  getBlockByHash,
  sendTransaction,
  getRawTransactionById,
  getRawBlockByHash,
  getRawBlockByHeight,
};
