/**
 * @module usernameIndex
 * This module is a temporary solution until proper index in dashcore is provided.
 */

const union = require('lodash/union');
const EventEmitter = require('events');
const dashcore = require('../api/dashcore').rpc;
const log = require('../log');

let usernameCache = [];
const userCache = {};
let lastSeenBlock = 1;

let isUpdating = false;

const events = new EventEmitter();

async function processBlock(blockHeight) {
  const blockHash = await dashcore.getBlockHash(blockHeight);
  const block = await dashcore.getBlock(blockHash);
  let nextBlockExists = false;
  if (block) {
    log.info(`Processing block ${block.height}`);
    nextBlockExists = !!block.nextblockhash;
    const transactionHashes = block.tx;
    let users = await Promise.all(transactionHashes.map(transactionHash => dashcore
      .getUser(transactionHash)
      .catch(() => null)));
    users = users.filter(user => !!user && !!user.uname);
    const usernamesInBlock = users.map(user => user.uname);

    // Updating full user index
    users.forEach((user) => {
      /*
      { uname: 'dmau6',
      regtxid: '3323acb1245133c63435155a8941e4a12f973358d170a4162793fec138a9b466',
      pubkeyid: 'd7d295e04202cc652d845cc51762dc64a5fd2bdc',
      credits: 1000000,
      data: '0000000000000000000000000000000000000000000000000000000000000000',
      state: 'open',
      subtx:
       [ '3323acb1245133c63435155a8941e4a12f973358d170a4162793fec138a9b466' ],
      transitions: [] }
       */
      userCache[user.regtxid] = user;
    });

    if (usernamesInBlock.length) {
      usernameCache = union(usernameCache, usernamesInBlock);
      log.info(`${usernamesInBlock.length} usernames added to cache: ${usernamesInBlock.join(', ')}`);
    } else {
      log.info('No usernames found.');
    }
  }
  events.emit('block_processed', nextBlockExists);
}

function updateUsernameIndex() {
  if (isUpdating) {
    log.info('Can\'t start updating index until previous update is finished');
    return Promise.resolve();
  }
  isUpdating = true;
  log.info('Updating username index...');
  return new Promise((resolve, reject) => {
    function blockHandler(isNextBlockExists) {
      if (isNextBlockExists) {
        lastSeenBlock += 1;
        processBlock(lastSeenBlock).catch(reject);
      } else {
        isUpdating = false;
        log.info('Update finished.');
        events.removeListener('block_processed', blockHandler);
        resolve();
      }
    }
    events.on('block_processed', blockHandler);
    processBlock(lastSeenBlock).catch(reject);
  });
}

async function searchUsernames(pattern) {
  // await updateUsernameIndex();
  return usernameCache.filter(username => !!username.match(pattern));
}

function getUserById(userId) {
  return userCache[userId];
}

function subscribeToZmq(zmqClient) {
  zmqClient.on(zmqClient.topics.hashblock, () => {
    try {
      updateUsernameIndex().catch((e) => {
        isUpdating = false;
        log.warn('Update finished with an error:');
        log.error(e);
      });
    } catch (e) {
      log.error(e);
    }
  });
}

module.exports = {
  searchUsernames,
  getUserById,
  subscribeToZmq,
  updateUsernameIndex,
};
