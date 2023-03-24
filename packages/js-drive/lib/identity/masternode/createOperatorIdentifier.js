const { hash } = require('@dashevo/dpp/lib/util/hash');

/**
 * @param {SimplifiedMNListEntry} smlEntry
 * @param {WebAssembly.Instance} dppWasm
 */
function createOperatorIdentifier(dppWasm, smlEntry) {
  const operatorPubKey = Buffer.from(smlEntry.pubKeyOperator, 'hex');

  return dppWasm.Identifier.from(
    hash(
      Buffer.concat([
        Buffer.from(smlEntry.proRegTxHash, 'hex'),
        operatorPubKey,
      ]),
    ),
  );
}

module.exports = createOperatorIdentifier;
