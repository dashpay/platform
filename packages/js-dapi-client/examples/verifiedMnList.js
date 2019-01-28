const { getVerifiedMnList } = require('../src/Helpers');

const log = console;

// 3 Snapshots of dapinet devnet includig 1 mn deletion  + 1 mn insertion
function sample1() {
  // Core bug: base block 0000000000000000000000000000000000000000000000000000000000000000
  // is higher then block... Using block 1 as nullhash until this is fixed
  const nullhash = '3f4a8012763b1d9b985cc77b0c0bca918830b1ef7dd083665bdc592c2cd31cf6';
  const allMnsActiveAtHash = '000004543e350b99f43114fe0bf649344a28f4fde6785d80e487d90689ae3918';
  const deletedMnHash = '00000f5da94df7df6d8715e149467a5e859fe5db48366a68ab94dadc389097e7';

  getVerifiedMnList(nullhash, [], allMnsActiveAtHash)
    .then((res) => {
      log.info('\nMnList @ block 2896 - 3 mns valid:');
      log.info(JSON.stringify(res.mnList, null, 4));
      return getVerifiedMnList(allMnsActiveAtHash, res.mnList, deletedMnHash);
    }).then((res) => {
      log.info('\nMnList @ block 2897 - 2 mns valid, 1 removed:');
      log.info(JSON.stringify(res.mnList, null, 4));
      return getVerifiedMnList(deletedMnHash, res.mnList);
    }).then((res) => {
      log.info('\nMnList @ latest block - deleted mn added back at block 2904:');
      log.info(JSON.stringify(res.mnList, null, 4));
    });
}

// get latest list every 2.5mins and check if valid
function sample2() {
  let lastTargetHash = '3f4a8012763b1d9b985cc77b0c0bca918830b1ef7dd083665bdc592c2cd31cf6';

  setInterval(() => {
    getVerifiedMnList(lastTargetHash, [])
      .then((res) => {
        if (res.valid) {
          log.info(`MN List at block: ${res.targetHash} contains ${res.mnList.length} active MN's (including PoSe banned)`);
          lastTargetHash = res.targetHash;
        } else {
          log.info(`No valid proofs found for mnlist at block ${res.targetHash} - retry at differrent node`);
        }
      });
  }, 2.5 * 1000 * 60);
}

sample1();
sample2();
