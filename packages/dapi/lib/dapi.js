const AuthService = require('./authService/authService');
const Insight = require('./insight/insight');
const Server = require('./server/server');
const Quorum = require('./quorum/quorum');
const Rpc = require('./rpc/rpc');
const { Logger } = require('./utils');

class Dapi {
    constructor (config) {
        this.logger = new Logger();
        this.logger.level = this.logger.VERBOSE;
        this.config = config;
        this.insight = new Insight(this);
        this.quorum = new Quorum(this);
        this.authService = new AuthService(this);
        this.rpcService = new Rpc(this);
        this.server = new Server(this);
    }
}
module.exports = Dapi;

//Override node v8 promises for now
global.Promise = require("bluebird");
