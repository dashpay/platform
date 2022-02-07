const { EventEmitter } = require('events');

class AppMediator extends EventEmitter {
}

AppMediator.events = {
  chainlock: 'chainlock',
  hashblock: 'hashblock',
};

module.exports = AppMediator;
