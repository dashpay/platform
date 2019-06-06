/**
 * @module usernameIndex
 * This module is a temporary solution until proper index in dashcore is provided.
 */

const union = require('lodash/union');
const EventEmitter = require('events');

let dashcoreRpcClient;
let logger;

let usernameCache = [];
const userCache = {};
let lastSeenBlock = 0;

let isUpdating = false;

const events = new EventEmitter();

let lockProcessBlock;

class Lock {
  constructor() {
    this.locked = false;
    this.waiting = [];
  }

  lock() {
    const unlock = () => {
      let nextResolve;
      if (this.waiting.length > 0) {
        nextResolve = this.waiting.pop(0);
        nextResolve(unlock);
      } else {
        this.locked = false;
      }
    };
    if (this.locked) {
      return new Promise((resolve) => {
        this.waiting.push(resolve);
      });
    }
    this.locked = true;
    return new Promise((resolve) => {
      resolve(unlock);
    });
  }
}


async function processBlock(blockHeight) {
  lockProcessBlock = await new Lock().lock();
  const blockHash = await dashcoreRpcClient.getBlockHash(blockHeight);
  const block = await dashcoreRpcClient.getBlock(blockHash);
  let nextBlockExists = false;
  if (block) {
    nextBlockExists = !!block.nextblockhash;
    const transactionHashes = block.tx;
    let users = await Promise.all(transactionHashes.map(transactionHash => dashcoreRpcClient
      .getUser(transactionHash)
      .catch(() => null)));
    users = users.filter(user => !!user && !!user.uname);
    const usernamesInBlock = users.map(user => user.uname);

    // Updating full user index
    users.forEach((user) => {
      userCache[user.regtxid] = user;
    });

    if (usernamesInBlock.length) {
      usernameCache = union(usernameCache, usernamesInBlock);
      logger.debug(`${usernamesInBlock.length} usernames added to cache: ${usernamesInBlock.join(', ')}`);
    }
    await lockProcessBlock();
  }

  events.emit('block_processed', nextBlockExists);
}

function updateUsernameIndex() {
  if (isUpdating) {
    logger.info('Can\'t start updating index until previous update is finished');
    return Promise.resolve();
  }
  isUpdating = true;
  logger.info('Updating username index');
  return new Promise((resolve, reject) => {
    function blockHandler(isNextBlockExists) {
      if (isNextBlockExists) {
        lastSeenBlock += 1;
        processBlock(lastSeenBlock).catch(reject);
      } else {
        logger.info('Username index updated');
        events.removeListener('block_processed', blockHandler);
        isUpdating = false;
        resolve();
      }
    }
    events.on('block_processed', blockHandler);
    processBlock(lastSeenBlock).catch(reject);
  });
}

async function searchUsernames(pattern) {
  return usernameCache.filter(username => !!username.match(pattern));
}

function getUserById(userId) {
  return userCache[userId];
}

function safeUpdateUsernameIndex() {
  try {
    updateUsernameIndex().catch((e) => {
      logger.warn('User index update finished with an error:');
      logger.error(e);
      lastSeenBlock -= 1;
      isUpdating = false;
      (async () => {
        try {
          await lockProcessBlock();
        } catch (err) {
          logger.debug(err);
        }
      })();
    });
  } catch (e) {
    logger.error(e);
  }
}

function start({ dashCoreRpcClient: rpcClient, dashCoreZmqClient, log }) {
  dashcoreRpcClient = rpcClient;
  logger = log;
  safeUpdateUsernameIndex();
  dashCoreZmqClient.on(dashCoreZmqClient.topics.hashblock, safeUpdateUsernameIndex);
}

module.exports = {
  searchUsernames,
  getUserById,
  start,
  updateUsernameIndex,
};
