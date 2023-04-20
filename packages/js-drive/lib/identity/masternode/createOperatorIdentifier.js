const Identifier = require('@dashevo/dpp/lib/identifier/Identifier');
const { hash } = require('@dashevo/dpp/lib/util/hash');

/**
 * @param {SimplifiedMNListEntry} smlEntry
 */
function createOperatorIdentifier(smlEntry) {
  const operatorPubKey = Buffer.from(smlEntry.pubKeyOperator, 'hex');

  return Identifier.from(
    hash(
      Buffer.concat([
        Buffer.from(smlEntry.proRegTxHash, 'hex'),
        operatorPubKey,
      ]),
    ),
  );
}

module.exports = createOperatorIdentifier;
