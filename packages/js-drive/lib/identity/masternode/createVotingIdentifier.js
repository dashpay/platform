const { hash } = require('@dashevo/dpp/lib/util/hash');
const Identifier = require('@dashevo/dpp/lib/identifier/Identifier');
const Address = require('@dashevo/dashcore-lib/lib/address');

/**
 * @param {SimplifiedMNListEntry} smlEntry
 */
function createVotingIdentifier(smlEntry) {
  const votingPubKey = Buffer.from(
    Address.fromString(smlEntry.votingAddress).hashBuffer,
    'hex',
  );

  return Identifier.from(
    hash(
      Buffer.concat([
        Buffer.from(smlEntry.proRegTxHash, 'hex'),
        votingPubKey,
      ]),
    ),
  );
}

module.exports = createVotingIdentifier;
