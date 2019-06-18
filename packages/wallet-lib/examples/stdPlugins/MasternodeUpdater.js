const { StandardPlugin } = require('../../src/plugins/index');
const Dashcore = require('@dashevo/dashcore-lib');

const { Payload } = Dashcore.Transaction;
const { ProUpRevTxPayload } = Payload;

class MasternodeUpdater extends StandardPlugin {
  updateMasternode() {
    const proUpReTx = {
      version: 1,
      proTXHash: '01040eb32f760490054543356cff463865633439dd073cffa570305eb086f70e',
      reason: 0,
      inputsHash: '4f422948637072af5cdc211bb30fe96386c4935f64da82e6a855c6c9f3b37708',
      payloadSig: '48d6a1bd2cd9eec54eb866fc71209418a950402b5d7e52363bfb75c98e141175',
    };
    const payload = new ProUpRevTxPayload.fromJSON(validProUpRevTxPayloadJSON);

    const transaction = Dashcore
      .Transaction()
      .setType(Dashcore.Transaction.TYPES.TRANSACTION_PROVIDER_UPDATE_REGISTRER)
      .setExtraPayload(payload);

    const serialized = transaction.serialize();
  }
}
module.exports = MasternodeUpdater;
