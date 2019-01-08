const dashcore = require('@dashevo/dashcore-lib');

// from dashcore/lib/block/block.js
function getMerkleTree(tree) {
  let j = 0;
  for (let size = tree.length; size > 1; size = Math.floor((size + 1) / 2)) {
    for (let i = 0; i < size; i += 2) {
      const i2 = Math.min(i + 1, size - 1);
      const buf = Buffer.concat([tree[j + i], tree[j + i2]]);
      tree.push(dashcore.crypto.Hash.sha256sha256(buf));
    }
    j += size;
  }

  return tree;
}

function calculateMnListMerkleRoot(mnList) {
  return getMerkleTree(mnList.sort((m1, m2) => m1.proRegTxHash > m2.proRegTxHash)
    .map(m => new dashcore.SimplifiedMNListEntry(m).getHash()))
    .slice(-1)[0]
    .toString('hex');
}

module.exports = calculateMnListMerkleRoot;
