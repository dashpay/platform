const AuthService = require('./authService/authService');
const Server = require('./server/server');
const Quorum = require('./quorum/quorum');
const { Logger } = require('./utils');
const insight = require('./insight');
const config = require('./config');

class Dapi {
  constructor() {
    this.logger = new Logger();
    this.logger.level = this.logger.VERBOSE;
    this.config = config;
    this.insight = insight;
    this.quorum = new Quorum(this);
    this.authService = new AuthService(this);
    this.server = new Server(this);
  }
}
module.exports = Dapi;

// Override node v8 promises for now
global.Promise = require('bluebird');
