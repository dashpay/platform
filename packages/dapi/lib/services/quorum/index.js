const qDash = require('@dashevo/quorums');
const dashcore = require('../../api/dashcore/rpc');
const logger = require('../../log');
const insight = require('../../api/insight');

const heartbeatInterval = 10;

let isHeartBeat = false;

const quorumService = {
  start(dashcoreZmqClient) {
    const newBlockEvent = dashcoreZmqClient.topics.hashblock;
    isHeartBeat = false;
    logger.debug(`- Init Quorum (heartbeat interval = ${heartbeatInterval} blocks)`);
    /* TODO: error handling for when dapi is started before MN is
    synced and therefore fails to connect with zmq */

    dashcoreZmqClient.on(newBlockEvent, async (msg) => {
      const hash = msg.toString('hex');
      const height = await dashcore.getCurrentBlockHeight();
      // let's see if we have a new heartbeat and need to migrate/join new quorum
      isHeartBeat = height % heartbeatInterval === 0;
      logger.debug(newBlockEvent, msg, hash, height, isHeartBeat);
      if (isHeartBeat) {
        // here comes the action!
        this.migrateClients();
        this.joinQuorum();
      }
    });
  },

  migrateClients() {
    logger.debug('migrate connected clients');
    // TODO: whatever we need to migrate our connected clients
  },

  async joinQuorum() {
    logger.debug('join new Quorum');
    // TODO: fix quorum-dash. qDash.getQuorum(quorumData) causes error
    const quorumData = await this.getQuorum();
    logger.debug(quorumData);
  },

  async getQuorumHash() {
    const bestHeight = await dashcore.getCurrentBlockHeight();
    return dashcore.getHashFromHeight(qDash.getRefHeight(bestHeight));
  },

  async getQuorum() {
    const [mnList, refHash] = await Promise.all([insight.getMasternodesList(),
      this.getQuorumHash()]);
    return qDash.getQuorum(mnList, refHash);
  },

  async isValidQuorum(body) {
    const [mnList, refHash, refAddr] = await Promise.all([
      insight.getMasternodesList(),
      this.getQuorumHash(),
      insight.getTransactionFirstInputAddress(body.data.txId),
    ]);
    const quorumData = { mnList, refHash, refAddr };
    return qDash.validate(body.data, body.signature, quorumData);
  },
};

module.exports = quorumService;
