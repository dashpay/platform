// Todo Extract each usecase into a helper function
const { SpvChain } = require('@dashevo/dash-spv');
const { MerkleProof } = require('@dashevo/dash-spv');
const dashcore = require('@dashevo/dashcore-lib');
const Api = require('../');
const helpers = require('../src/Helpers');

let validMnList = [];

// Height used for poc (to save syning time)
const pocBestHeight = 2896;

// Go back 20 blocks
// Todo: DGW not allowing more than 24 blocks, low difficulty regtest problem
const pocGenesis = pocBestHeight - 20;

let nullHash;
let api = null;
let headerChain = null;

const log = console;

async function logOutput(msg, delay = 50) {
  log.info(`${msg}`);
  await new Promise(resolve => setTimeout(resolve, delay));
}

// ==== Client initial state

async function init() {
  api = new Api();
  // using genesis as nullhash as core is bugged
  nullHash = await api.getBlockHash(0);
}

// ==== Client initial state

// ==== Build HeaderChain

async function setTrustedMnLists() {
  const latestHash = await api.getBlockHash(await api.getBestBlockHeight());
  const trustedMnListDiff = await api.getMnListDiff(nullHash, latestHash);
  const trustedMnList = helpers.constructMnList([], trustedMnListDiff);

  // todo - change dapi architecture to accomodate setting new lists
  // api.MNDiscovery.masternodeListProvider.masternodeList.concat(trustedMnList);

  await logOutput(`Set Trusted MnList: mnlist length = ${trustedMnList.length}`);
}

async function getValidatedHeaderchain() {
  const dapinetGenesisHash = await api.getBlockHash(pocGenesis);
  const dapinetGenesisHeader = await api.getBlockHeader(dapinetGenesisHash);
  dapinetGenesisHeader.prevHash = '0000000000000000000000000000000000000000000000000000000000000000';
  dapinetGenesisHeader.bits = +(`0x${dapinetGenesisHeader.bits}`);
  const numConfirms = 10000;

  headerChain = new SpvChain('custom_genesis', numConfirms, dapinetGenesisHeader);

  const maxHeaders = 24;
  for (let i = pocGenesis + 1; i <= pocBestHeight; i += maxHeaders) {
    /* eslint-disable-next-line no-await-in-loop */
    const newHeaders = await api.getBlockHeaders(i, maxHeaders);
    headerChain.addHeaders(newHeaders);
  }

  // NOTE: query a few nodes by repeating the process to make sure you on the longest chain
  // headerChain instance will automatically follow the longest chain, keep track of orphans, etc
  // implementation detail @ https://docs.google.com/document/d/1jV0zCie5rVbbK9TbhkDUbbaQ9kG9oU8XTAWMVYjRc2Q/edit#heading=h.trwvf85zn0se

  await logOutput(`Got headerchain with longest chain of length ${headerChain.getLongestChain().length}`);
}

async function validateCheckpoints(checkpoints) {
  if (checkpoints.every(cp => headerChain.getLongestChain().map(h => h.hash).includes(cp))) {
    await logOutput(`Checkpoints valid on headerChain ${headerChain.getLongestChain().length}`);
  } else {
    await logOutput('INVALID CHECKPOINT! please query more headers from other dapi nodes');
  }
}

async function BuildHeaderChain() {
  await setTrustedMnLists();
  await getValidatedHeaderchain();

  // select 2 random from chain, in production this will be hardcoded
  const checkpoints = headerChain.getLongestChain()
    .map(h => h.hash)
    .sort(() => 0.5 - Math.random()); // 1 liner (sub optimal) shuffle hack
  // .slice(0, 2);

  await validateCheckpoints(checkpoints);

  logOutput('Build HeaderChain complete');
}

// ==== Build HeaderChain

// ==== Get Verified MnList

function validateDiffListProofs(mnlistDiff, header, newList) {
  // Todo: pending core RPC bug currently not returning these proofs
  // rem next line when proofs available
  return mnlistDiff && header && newList && dashcore && MerkleProof;

  // Add this code back when proofs available.
  // return MerkleProof.validateMnProofs(
  //   header,
  //   mnlistDiff.merkleFlags,
  //   mnlistDiff.merkleHashes,
  //   mnlistDiff.totalTransactions,
  //   mnlistDiff.cbTx.hash,
  // ) &&
  // MerkleProof.validateMnListMerkleRoot(mnlistDiff.mnlistMerkleRoot, newList);
}

function constructMnList(originalMnList, diffList) {
  const replacements = diffList.mnList.map(x => x.proRegTxHash);
  return originalMnList
    .filter(mn => !diffList.deletedMNs.includes(mn.proRegTxHash)) // remove deleted
    .filter(mn => !replacements.includes(mn.proRegTxHash)) // to be replaced
    .concat(diffList.mnList) // replace
    .sort((itemA, itemB) => itemA.proRegTxHash > itemB);
}

async function getMnListDiff() {
  // Suppose to use nullhash as offset for full list but core bug prevent this currently
  // so using block 1 which will have same effect
  const offSetHash = await api.getBlockHash(1);
  const targetHash = await api.getBlockHash(pocBestHeight);
  const diffList = await api.getMnListDiff(offSetHash, targetHash);

  return diffList;
}

async function GetVerifiedMnList() {
  const diffList = await getMnListDiff();
  const trustedMnList = constructMnList(validMnList, diffList);
  // Todo: replace pocBestHeight with diffList.cbTx.height (pocBestHeight should be same val)
  const cbTxHeader = await headerChain.getHeader(await api.getBlockHash(pocBestHeight));
  const proofsIsValid = validateDiffListProofs(diffList, cbTxHeader, trustedMnList);

  if (proofsIsValid) {
    validMnList = [...trustedMnList];
    await logOutput(`Checkpoints valid on headerChain with tip ${headerChain.getTipHash()}`);
  } else {
    await logOutput('INVALID MNLIST! please query other dapi nodes');
  }
}

// ==== Get Verified MnList


async function start() {
  await init(); // Client Initial state
  await BuildHeaderChain();
  await GetVerifiedMnList();
}

start();
