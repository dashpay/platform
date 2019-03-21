class QuorumService {
  constructor({
    dashCoreRpcClient, dashCoreZmqClient, log, heartbeatInterval = 10,
  }) {
    this.dashCoreRpcClient = dashCoreRpcClient;
    this.dashCoreZmqClient = dashCoreZmqClient;
    this.log = log;
    this.heartbeatInterval = heartbeatInterval;
    this.isHeartBeat = false;
  }

  start() {
    const {
      log, dashCoreRpcClient, dashCoreZmqClient, heartbeatInterval,
    } = this;
    const newBlockEvent = dashCoreZmqClient.topics.hashblock;
    this.isHeartBeat = false;
    log.debug(`- Init Quorum (heartbeat interval = ${heartbeatInterval} blocks)`);
    /* TODO: error handling for when dapi is started before MN is
    synced and therefore fails to connect with zmq */

    dashCoreZmqClient.on(newBlockEvent, async (msg) => {
      const hash = msg.toString('hex');
      const height = await dashCoreRpcClient.getBestBlockHeight();
      // let's see if we have a new heartbeat and need to migrate/join new quorum
      this.isHeartBeat = height % heartbeatInterval === 0;
      log.debug(newBlockEvent, msg, hash, height, this.isHeartBeat);
      if (this.isHeartBeat) {
        this.migrateClients();
        this.joinQuorum();
      }
    });
  }

  migrateClients() {
    this.log.debug('migrate connected clients');
  }

  async joinQuorum() {
    this.log.debug('join new Quorum');
  }
}

module.exports = QuorumService;
