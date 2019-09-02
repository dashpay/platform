const Transaction = require('@dashevo/dashcore-lib/lib/transaction');

class StateTransition extends Transaction {}

StateTransition.TYPES = Transaction.TYPES;

module.exports = StateTransition;
