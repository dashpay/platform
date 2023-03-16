const { hash } = require('@dashevo/dpp/lib/util/hash');
const Address = require('@dashevo/dashcore-lib/lib/address');

/**
 * @param {SimplifiedMNListEntry} smlEntry
 * @param {WebAssembly.Instance} dppWasm
 */
function createVotingIdentifier(smlEntry, dppWasm) {
  const votingPubKeyHash = Address.fromString(smlEntry.votingAddress).hashBuffer;

  return dppWasm.Identifier.from(
    hash(
      Buffer.concat([
        Buffer.from(smlEntry.proRegTxHash, 'hex'),
        votingPubKeyHash,
      ]),
    ),
  );
}

module.exports = createVotingIdentifier;
