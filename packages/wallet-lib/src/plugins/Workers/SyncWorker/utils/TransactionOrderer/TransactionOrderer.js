/**
 * Allows to manage transactions and canonically order them by
 * having all transactions linked by their prevTxId previous inputs
 *
 * Reminder : TX has 1 or + inputs, and 1 or + outputs.
 * We may encounter multiples cases :
 *  - We add an isolated transaction which input's prevTxId are not in our list
 *  - We add a tx which inputs has a prevTxId in (predecesasor) : We add it after it.
 *  - We add a tx which output are inputs of a tx in the list (successor) : we add it before it
 *  - We add a tx which has both predecessor before an a successor after : We add in between
 *  - We add a tx which has a predecessor after it's successor (because of addition timeline) :
 *     this transaction's successors and right members are pushed after the predecessor
 *     and we add our tx in between.
 *  - We add a tx that has similar case than above but on multiples input and outputs.
 */
class TransactionOrderer {
  constructor() {
    this.transactions = [];
    // Only uses is to speed up lookups
    this.transactionIds = [];
  }
}

TransactionOrderer.prototype.insert = require('./methods/insert');
TransactionOrderer.prototype.insertAtPos = require('./methods/insertAtPos');
TransactionOrderer.prototype.lookupByTransactionHash = require('./methods/lookupByTransactionHash');
TransactionOrderer.prototype.lookupInputsPredecessors = require('./methods/lookupInputsPredecessors');
TransactionOrderer.prototype.lookupTxIdSuccessors = require('./methods/lookupTxIdSuccessors');
TransactionOrderer.prototype.reset = require('./methods/reset');

module.exports = TransactionOrderer;
