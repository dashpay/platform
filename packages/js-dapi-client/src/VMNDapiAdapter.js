/**
 * Copyright (c) 2017-present, Dash Core Team
 *
 * This source code is licensed under the MIT license found in the
 * COPYING file in the root directory of this source tree.
 */
const BitcoreLib = require('@dashevo/dashcore-lib');
const Schema = require('@dashevo/dash-schema/lib');
const DAPI = require('./index');
const EventEmitter = require('eventemitter2');

const { Registration } = BitcoreLib.Transaction.SubscriptionTransactions;
const { TransitionHeader, TransitionPacket } = BitcoreLib.StateTransition;

/**
 * Virtual HTTPS Interface for DAPI test-stack module
 * @interface VMNDAPIAdapter
 */
class VMNDAPIAdapter {
  constructor(options) {
    this.events = new EventEmitter();
    this.DAPI = new DAPI(options ? options.seeds : null);
  }

  /**
   * Handles NewBlock ZMQ message from DashCore
   * @param {object} blockInfo info object
   */
  _onNewBlock(blockInfo) {
    this.bestBlockInfo.height = blockInfo.height;
    this.bestBlockInfo.hash = blockInfo.hash;

    this.events.emit('newBlock', this.bestBlockInfo);
  }

  /**
   * Create a Blockchain blockchainuser via a SubTX
   * @param {json} obj Raw subtx
   */
  async CreateUser(obj) {
    // this.log('Signup blockchainuser', obj.subtx.uname);
    const regTx = new Registration(obj);
    const regTxId = await this.DAPI.sendRawTransaction(regTx.serialize());
    // Mine 1 block to confirm regtx. Command available only in regtest mode.
    await this.DAPI.generate(1);
    return regTxId;
  }

  /**
   * Returns a single BlockchainUser Schema object for the specified username
   * @param {string} uname Blockchain Username
   * @memberof DAPI
   */
  async GetUserByName(uname) {
    if (!Schema.validate.username(uname)) {
      return null;
    }
    return this.DAPI.getUserByName(uname);
  }

  /**
   * Returns a single BlockchainUser Schema object for the specified uid
   * @param {string} uid
   */
  async GetUserById(uid) {
    return this.DAPI.getUserById(uid);
  }

  /**
   * Search for blockchain users who match a given search pattern
   * @param {string} pattern - search string
   * @returns {array} Array of matching blockchain blockchainuser accounts
   */
  async SearchUsers(pattern) {
    return this.DAPI.searchUsers(pattern);
  }

  async GetDap(dapid) {
    return this.DAPI.fetchDapContract(dapid);
  }

  async SearchDaps(pattern) {
    return this.DAPI.searchDapContracts(pattern);
  }

  /**
   * Returns a users current dataset from DashDrive
   * @param {string} dapid Blockchain Username
   * @param {string} uid Hash of the DAP Schema
   * @memberof DAPI
   */
  async GetDapSpace(dapid, uid) {
    return this.DAPI.getUserDapSpace(dapid, uid);
  }

  async GetDapContext(dapid, uid) {
    return this.DAPI.getUserDapContext(dapid, uid);
  }

  /**
   * Updates a Blockchain blockchainuser's DAP data in DashCore (hash) and DashDrive (data).
   * The DAP space to use is determined from the dapid (hash) in the provided transition packet
   * @param {object} ts - State transition header
   * @param {object} tsp - State transition packet
   * @memberof DAPI
   */
  async UpdateDapSpace(ts, tsp) {
    if (Schema.object.validate(ts).valid === false) {
      throw new Error('Invalid tsheader');
    }

    if (Schema.object.validate(tsp).valid === false) {
      throw new Error('Invalid tpacket');
    }

    // TODO
    // Here we need to send data to dashdrive - it is DAPI responsibility.
    const header = new TransitionHeader(ts);
    const packet = new TransitionPacket(tsp);
    const tsid = await this.DAPI.sendRawTransition(
      header.serialize(),
      packet.toHexString(),
    );

    // Mine 1 block to confirm state transition. This command available only in regtest mode.
    await this.DAPI.generate(1);

    return tsid;
  }
}

module.exports = VMNDAPIAdapter;
