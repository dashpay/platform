const { EventEmitter } = require('events');

class AppMediator extends EventEmitter {
  events = {
    chainlock: 'chainlock',
    hashblock: 'hashblock',
  }
}

module.exports = AppMediator;
