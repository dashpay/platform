const Client = require('../');
// Add this ref back when proofs available.
// const { MerkleProof } = require('dash-spv');

const client = new Client();

function getBestBlockHash() {
  return client.getBestBlockHeight()
    .then(height => client.getBlockHash(height));
}

function constructMnList(originalMnList, diffList) {
  const replacements = diffList.mnList.map(x => x.proRegTxHash);
  return originalMnList
    .filter(mn => !diffList.deletedMNs.includes(mn.proRegTxHash)) // remove deleted
    .filter(mn => !replacements.includes(mn.proRegTxHash)) // to be replaced
    .concat(diffList.mnList) // replace
    .sort((itemA, itemB) => itemA.proRegTxHash > itemB);
}

function validateDiffListProofs(mnlistDiff, header, newList) {
  // Todo: pending core RPC bug currently not returning these proofs
  // rem next line when proofs available
  return mnlistDiff && header && newList;

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

async function getVerifiedMnList(offSetHash, lastSyncedMnList, _targetHash) {
  const targetHash = _targetHash || await getBestBlockHash();
  const refHeader = await client.getBlockHeader(targetHash);
  const difflist = await client.getMnListDiff(offSetHash, targetHash);
  const newList = constructMnList(lastSyncedMnList, difflist);

  return {
    mnList: newList,
    valid: validateDiffListProofs(difflist, refHeader, newList),
    cbTxHash: '//todo pending core bug: difflist.cbTx.hash',
    targetHash,
  };
}

module.exports = {
  getVerifiedMnList,
  constructMnList,
};
