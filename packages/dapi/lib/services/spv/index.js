// temporary implementation (will likely not use p2p in future)
const p2p = require('@dashevo/dashcore-p2p');
const hash = require('object-hash');
const log = require('../../log');
const Cache = require('../caching/spvSimpleCache');
const config = require('../../config');

const getCorrectedHash = (reversedHashObj) => {
  const clone = Buffer.alloc(32);
  reversedHashObj.copy(clone);
  return clone.reverse().toString('hex');
};

const clearDisconnectedClientBloomFilters = ({
  clients, currentTime,
  hasDisconnectedThresholdInMsec,
}) => {
  if (!clients.length) {
    return [];
  }

  return clients.filter((client) => {
    if (currentTime - client.lastSeen >= hasDisconnectedThresholdInMsec) {
      client.peer.sendMessage(client.peer.messages.FilterClear(client.filter));
      return false;
    }
    return true;
  });
};

class SpvService {
  constructor() {
    this.config = config.dashcore.p2p;
    this.clients = [];
    this.cache = new Cache();
    const { bloomFilterPersistenceTimeout } = config;
    setInterval(() => {
      this.clients = clearDisconnectedClientBloomFilters({
        clients: this.clients,
        currentTime: Date.now(),
      });
      this.cache.clearInactiveClients(this.clients.map(client => client.filterHash));
    }, bloomFilterPersistenceTimeout);
  }

  updateLastSeen(filter) {
    this.clients.find(client => client.filterHash === hash(filter)).lastSeen = Date.now();
  }

  createNewClient(filter) {
    const client = {
      filter,
      filterHash: hash(filter),
      peer: new p2p.Peer(this.config),
      lastSeen: Date.now(),
    };
    this.clients.push(client);

    const { peer } = client;
    peer.connect();
    return new Promise((resolve, reject) => {
      peer.once('ready', () => {
        resolve(client);
      });

      peer.once('disconnect', () => {
        log.info('Peer disconnected...');
        reject(new Error('Not able to establish p2p connection to dashcore'));
      });
    });
  }

  initListeners(client, filter) {
    const peer = this.getPeerFromClients(filter);
    peer.on('inv', (message) => {
      message.inventory.forEach((m) => {
        const cHash = getCorrectedHash(m.hash);
        switch (m.type) {
          case 1:
            peer.sendMessage(peer.messages.GetData.forTransaction(cHash));
            break;
          case 2:
            peer.sendMessage(peer.messages.GetData.forFilteredBlock(cHash));
            break;
          default:
        }
      });
    });

    peer.on('tx', (message) => {
      this.cache.set(client.filterHash, message.transaction);
      log.info(`DAPI: tx ${message.transaction.hash} added to cache`);
    });

    peer.on('merkleblock', (message) => {
      // Rudimentary assumption of which blocks contains merkle proofs
      // Discussion: https://dashpay.atlassian.net/browse/EV-847
      if (message.merkleBlock.hashes.length > 1) {
        this.cache.set(client.filterHash, message.merkleBlock);
        log.info(`DAPI: merkleblock with ${message.merkleBlock.hashes.length} hash(es) added to cache`);
      }
    });
  }

  hasPeerInClients(filter) {
    const filterHash = hash(filter);
    return this.clients.filter(client => client.filterHash === filterHash).length > 0;
  }

  getPeerFromClients(filter) {
    const filterHash = hash(filter);
    return this.clients.filter(client => client.filterHash === filterHash)[0].peer;
  }

  loadBloomFilter(filter) {
    return new Promise((resolve, reject) => {
      const filterHash = hash(filter);
      if (!this.hasPeerInClients(filter)) {
        this.createNewClient(filter)
          .then((client) => {
            this.initListeners(client, filter);
            const peer = this.getPeerFromClients(filter);
            peer.sendMessage(peer.messages.FilterLoad(filter));
            this.updateLastSeen(filter);
            resolve(`Created new peer with bloomfilter hash: ${filterHash}`);
          })
          .catch(err => reject(err));
      } else {
        resolve(`Filter with: ${filterHash} already loaded`);
      }
    });
  }

  clearBoomFilter(filter) {
    if (this.hasPeerInClients(filter)) {
      const peer = this.getPeerFromClients(filter);
      peer.sendMessage(peer.clearBoomFilter(filter));
    } else {
      log.error('Attempting to clear a filter that has not been set');
    }
  }

  // Todo: rethink logic of using filter as client unique id
  addToBloomFilter(originalFilter, element) {
    this.updateLastSeen(originalFilter);
    if (this.hasPeerInClients(originalFilter)) {
      const peer = this.getPeerFromClients(originalFilter);
      peer.sendMessage(peer.addFilter(element));
    } else {
      log.error('No matching original filter. Please load a filter first');
    }
  }

  getSpvData(filter) {
    this.updateLastSeen(filter);
    return this.cache.get(hash(filter));
  }

  findDataForBlock(filter, blockHash) {
    this.updateLastSeen(filter);
    const peer = this.getPeerFromClients(filter);
    peer.sendMessage(peer.messages.GetData.forFilteredBlock(blockHash));
  }
}

module.exports = {
  SpvService,
  getCorrectedHash,
  clearDisconnectedClientBloomFilters,
};
