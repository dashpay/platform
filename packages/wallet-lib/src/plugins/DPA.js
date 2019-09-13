const DashPlatformProtocol = require('@dashevo/dpp');
const { Transaction } = require('@dashevo/dashcore-lib');
const StandardPlugin = require('./StandardPlugin');

// eslint-disable-next-line no-underscore-dangle
const _defaultOpts = {
  schema: null,
  verifyOnInjected: true,
  isValid: false,
};

class DPA extends StandardPlugin {
  constructor(opts = JSON.parse(JSON.stringify(_defaultOpts))) {
    const defaultOpts = JSON.parse(JSON.stringify(_defaultOpts));
    super({ type: 'DPA', ...opts });
    this.schema = (opts.schema !== undefined) ? opts.schema : defaultOpts.schema;
    this.verifyOnInjected = opts.verifyOnInjected !== undefined
      ? opts.verifyOnInjected
      : defaultOpts.verifyOnInjected;
    this.isValid = (opts.isValid !== undefined) ? opts.isValid : defaultOpts.isValid;
  }

  initDPP() {
    const dpp = new DashPlatformProtocol();
    const dpaName = this.name.toLowerCase();
    const { schema } = this;
    if (!schema) {
      throw new Error('Missing DPA Schema. Cannot init DPP');
    }
    const contract = dpp.contract.create(dpaName, schema);
    if (!dpp.contract.validate(contract)
      .isValid()) {
      throw new Error('Invalid DPA Contract');
    }
    dpp.setContract(contract);
    this.dpp = dpp;
  }

  async verifyDPA(transporter) {
    if (!this.schema) {
      throw new Error('Missing DPA Schema. Cannot verify');
    }
    if (!this.dpp) {
      this.initDPP();
    }
    const contractId = this.dpp.getContract().getId();
    console.log('Verifying DPA ID', contractId);

    if (!transporter || !transporter.fetchContract) {
      throw new Error('Require transporter to have a fetchContract method to verify DPA Contract');
    }
    try {
      await transporter.fetchContract(contractId);
      this.isValid = true;
      return this.isValid;
    } catch (e) {
      const isContractNotFoundError = new RegExp('Contract.*not.*found.*', 'g');
      if (isContractNotFoundError.test(e.message)) {
        throw new Error('Contract not present on the network. Did you `register`-ed it ? ');
      } else {
        throw e;
      }
    }
  }

  async register(buser, privateKey = null) {
    console.log(`Registering DPA : ${this.name}`);
    const creditFeeSet = 1000;
    if (!this.dpp) {
      this.initDPP();
    }
    const { dpp } = this;
    if (!buser) {
      throw new Error('A BUser Object is required to register (see @dashevo/dashpay-dpa)');
    }
    if (!buser.regtxid) {
      console.log(`'Registering DPA : ${this.name} - Missing regtxid, trying synchronize...`);
      try {
        await buser.synchronize();
      } catch (e) {
        console.error(e);
        throw new Error('Invalid BUser or inable to synchronize (regtxid missing.)');
      }
    }
    if (!buser.isOwned && !privateKey) {
      throw new Error('Either pass a owned buser or a private key to register the dpa');
    }
    const { regtxid, subtx } = buser;
    const signingKey = buser.privateKey || privateKey;
    if (!signingKey) throw new Error('A signingKey is required to sign the transaction');
    const dppContract = dpp.getContract();

    const stPacket = dpp.packet.create(dppContract);

    const hashPrevSubTx = (subtx.length === 0)
      ? regtxid
      : Array.from(subtx)
        .pop();

    const transaction = new Transaction()
      .setType(Transaction.TYPES.TRANSACTION_SUBTX_TRANSITION);

    transaction.extraPayload
      .setRegTxId(regtxid)
      .setHashPrevSubTx(hashPrevSubTx)
      .setHashSTPacket(stPacket.hash())
      .setCreditFee(creditFeeSet)
      .sign(signingKey);

    const txid = await this.transport.transport.sendRawTransition(
      transaction.serialize(),
      stPacket.serialize()
        .toString('hex'),
    );

    console.log(`DPA ${dppContract.name} Registered (txid ${txid}.)`);
    return txid;
  }
}

module.exports = DPA;
